//! Browse sessions that load a `ClusterRepository`'s credentials from another
//! namespace.
//!
//! A browse session Job runs beside the `Snapshot` it reads, and its pod loads
//! the repository's credential Secrets via `envFrom`, which only reads Secrets
//! in the pod's own namespace. A `ClusterRepository` usually keeps them in the
//! operator's namespace, so the session asks for copies instead: `kopiur_ops`
//! labels the Job `session-creds=projected`, names the copies in its `envFrom`
//! (`{job}-creds-{idx}`, [`kopiur_api::creds::browse_session_copy_name`]) and
//! annotates the `Snapshot` it was started for. The pod waits until the copies
//! exist; this reconciler writes them.
//!
//! The console and the person browsing never read or write a Secret, so the
//! operator is the only party that can, and it trusts nothing on the Job
//! beyond where to look. Before copying it re-checks the gates a backup's
//! projection passes, against live objects:
//!
//! * the annotated `Snapshot` exists in the Job's namespace and lives in a
//!   `ClusterRepository`, and the Job's controller ownerReference is that same
//!   repository (by uid), so a Job cannot borrow another repository's snapshot;
//! * the `ClusterRepository` allows projection (`credentialProjection.allowed`);
//! * the run that took the snapshot opted in (its pinned
//!   `status.resolved.credentialProjection`), so browsing never widens what the
//!   backup was allowed;
//! * the namespace is one the `ClusterRepository`'s `allowedNamespaces` admits.
//!
//! A closed gate is a Warning Event on the Job, and nothing is copied. The
//! reason also goes on the Job itself, as the
//! [`SESSION_CREDS_REFUSED_ANNOTATION`]: the person browsing can read a Job but
//! not its Events, and the session's wait reports it from there. A projection
//! that fails for a reason the user must fix (the operator's Secret RBAC, a
//! missing source) is written there too; a passing apiserver error is not, so
//! the wait keeps waiting through the retry. Nothing the gates read is watched,
//! so a refusal re-checks every [`REFUSAL_RECHECK`] while the session lives, and
//! fixing the gate lets the waiting pod start.
//!
//! Each copy is owned by the session Job, in the same namespace, so it goes
//! when the Job does: when the session is ended, at its TTL, or with the
//! repository. While the Job lives its `envFrom` keeps the copy in the sweep's
//! in-use set, and the sweep's terminal-copy pass only ever selects copies
//! owned by a `Snapshot`, so it never reaps one early.

use std::collections::BTreeMap;
use std::sync::Arc;

use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Namespace;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::runtime::controller::Action;
use kube::{Api, ResourceExt};

use kopiur_api::common::RepositoryKind;
use kopiur_api::consts::{SESSION_CREDS_REFUSED_ANNOTATION, SESSION_SNAPSHOT_ANNOTATION};
use kopiur_api::creds::{CredsSecretRef, mover_creds_secret_refs};
use kopiur_api::{ClusterRepository, Snapshot, validate};

use crate::context::Context;
use crate::error::{Error, Result, error_policy_for};
use crate::io::{self, CredsContext, CredsPrefix};

/// Why a session Job gets no copies. Each names what to change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The Job names no `Snapshot`.
    NoSnapshot,
    /// The named `Snapshot` is not in the Job's namespace.
    SnapshotNotFound {
        /// The annotated name.
        snapshot: String,
    },
    /// The `Snapshot` lives in a namespaced `Repository`, whose Secrets are
    /// never projected, or names no repository at all.
    NotAClusterRepository {
        /// The annotated name.
        snapshot: String,
    },
    /// The Job's controller ownerReference is not the snapshot's repository.
    NotThisRepository {
        /// The snapshot's `ClusterRepository`.
        repository: String,
    },
    /// The `ClusterRepository` does not exist.
    RepositoryNotFound {
        /// Its name.
        repository: String,
    },
    /// The `ClusterRepository` does not allow projection.
    OwnerNotAllowed {
        /// Its name.
        repository: String,
    },
    /// The run that took the snapshot did not opt in.
    ConsumerNotOptedIn {
        /// The snapshot's name.
        snapshot: String,
    },
    /// The namespace is not one the `ClusterRepository` admits.
    NamespaceNotAllowed {
        /// The Job's namespace.
        namespace: String,
        /// The `ClusterRepository`.
        repository: String,
    },
}

impl Refusal {
    /// The Event message: what was refused, and how to allow it.
    pub fn message(&self) -> String {
        let what = match self {
            Refusal::NoSnapshot => format!(
                "the session Job has no {SESSION_SNAPSHOT_ANNOTATION} annotation naming its \
                 Snapshot. Fix: start the session from kopiur (the console or kubectl kopiur), \
                 which sets it"
            ),
            Refusal::SnapshotNotFound { snapshot } => format!(
                "Snapshot {snapshot} is not in this namespace. Fix: browse a snapshot that \
                 exists here"
            ),
            Refusal::NotAClusterRepository { snapshot } => format!(
                "Snapshot {snapshot} is not in a ClusterRepository, and only a \
                 ClusterRepository's credentials are copied for a session. Fix: keep the \
                 Repository and its Secret in this namespace"
            ),
            Refusal::NotThisRepository { repository } => format!(
                "the session Job is not owned by ClusterRepository {repository}, which holds \
                 its snapshot. Fix: start the session from kopiur, which sets that owner"
            ),
            Refusal::RepositoryNotFound { repository } => format!(
                "ClusterRepository {repository} does not exist. Fix: browse a snapshot of a \
                 repository that does"
            ),
            Refusal::OwnerNotAllowed { repository } => format!(
                "ClusterRepository {repository} does not allow its credentials to be copied \
                 into other namespaces. Fix: set spec.credentialProjection.allowed: true on it"
            ),
            Refusal::ConsumerNotOptedIn { snapshot } => format!(
                "Snapshot {snapshot} was taken without credentialProjection, so its \
                 credentials are not copied for a session either. Fix: set \
                 spec.credentialProjection.enabled: true on its SnapshotPolicy, and browse a \
                 snapshot it takes after that"
            ),
            Refusal::NamespaceNotAllowed {
                namespace,
                repository,
            } => format!(
                "namespace {namespace} is not in ClusterRepository {repository}'s \
                 allowedNamespaces. Fix: add it there"
            ),
        };
        format!("Credentials not copied for this browse session: {what}.")
    }
}

/// **Pure.** The `ClusterRepository` a session Job's annotated snapshot lives
/// in, checked against the Job's own controller ownerReference (which
/// `kopiur_ops` sets to the repository it holds open).
pub fn session_repository(job: &Job, snapshot: &Snapshot) -> Result<String, Refusal> {
    let name = snapshot.name_any();
    let rref = kopiur_api::snapshot::repository_ref_for(snapshot)
        .filter(|r| r.kind == RepositoryKind::ClusterRepository)
        .ok_or(Refusal::NotAClusterRepository { snapshot: name })?;
    let owned = job.owner_references().iter().any(|o| {
        o.controller == Some(true) && o.kind == "ClusterRepository" && o.name == rref.name
    });
    if owned {
        Ok(rref.name)
    } else {
        Err(Refusal::NotThisRepository {
            repository: rref.name,
        })
    }
}

/// **Pure.** The credential Secrets to copy for a session Job in `namespace`,
/// once its snapshot and repository are known: the gates a backup's
/// projection passes, read off live objects.
pub fn session_gates(
    job: &Job,
    namespace: &str,
    snapshot: &Snapshot,
    repo: &ClusterRepository,
    namespace_labels: Option<&BTreeMap<String, String>>,
) -> Result<Vec<CredsSecretRef>, Refusal> {
    let repository = repo.name_any();
    // The owner reference names the repository; the uid proves it is this one
    // and not an earlier repository of the same name.
    let uid = repo.uid().unwrap_or_default();
    if !job
        .owner_references()
        .iter()
        .any(|o| o.controller == Some(true) && o.kind == "ClusterRepository" && o.uid == uid)
    {
        return Err(Refusal::NotThisRepository { repository });
    }
    if !repo
        .spec
        .credential_projection
        .as_ref()
        .is_some_and(|p| p.allowed)
    {
        return Err(Refusal::OwnerNotAllowed { repository });
    }
    let opted_in = snapshot
        .status
        .as_ref()
        .and_then(|s| s.resolved.as_ref())
        .and_then(|r| r.credential_projection.as_ref())
        .is_some_and(|p| p.enabled);
    if !opted_in {
        return Err(Refusal::ConsumerNotOptedIn {
            snapshot: snapshot.name_any(),
        });
    }
    if validate::validate_consumer_against_cluster_repo(
        namespace,
        &repository,
        &repo.spec.allowed_namespaces,
        namespace_labels,
    )
    .is_err()
    {
        return Err(Refusal::NamespaceNotAllowed {
            namespace: namespace.to_string(),
            repository,
        });
    }
    Ok(mover_creds_secret_refs(
        &repo.spec.backend,
        &repo.spec.encryption,
        None,
    ))
}

/// **Pure.** Whether a session Job is past needing credentials: finished, or
/// being deleted. Its copies go with it.
pub fn job_is_done(job: &Job) -> bool {
    job.metadata.deletion_timestamp.is_some()
        || job
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .is_some_and(|cs| {
                cs.iter()
                    .any(|c| (c.type_ == "Complete" || c.type_ == "Failed") && c.status == "True")
            })
}

/// **Pure.** The ownerReference each copy carries: the session Job itself, in
/// the same namespace, so garbage collection removes the copy with the Job.
pub fn copy_owner(job: &Job) -> OwnerReference {
    OwnerReference {
        api_version: "batch/v1".to_string(),
        kind: "Job".to_string(),
        name: job.name_any(),
        uid: job.uid().unwrap_or_default(),
        controller: Some(true),
        block_owner_deletion: Some(false),
    }
}

/// How soon a refused session is checked again. A session lives minutes, and
/// someone who just fixed a gate is waiting on it.
pub const REFUSAL_RECHECK: std::time::Duration =
    std::time::Duration::from_secs(kopiur_api::consts::SESSION_CREDS_RECHECK_SECS);

/// **Pure.** What to do after a refusal: look again soon, since fixing a gate
/// changes objects this controller does not watch.
pub fn after_refusal() -> Action {
    Action::requeue(REFUSAL_RECHECK)
}

/// **Pure.** The merge patch that makes the Job's refusal annotation say
/// `want` (`None` removes it), or `None` when it already does.
pub fn refusal_patch(job: &Job, want: Option<&str>) -> Option<serde_json::Value> {
    let current = job
        .annotations()
        .get(SESSION_CREDS_REFUSED_ANNOTATION)
        .map(String::as_str);
    (current != want).then(|| {
        serde_json::json!({
            "metadata": { "annotations": { SESSION_CREDS_REFUSED_ANNOTATION: want } }
        })
    })
}

/// Bring the refusal annotation to `want`. Best-effort: the Event says the same,
/// and a session that cannot see it still ends at its readiness deadline.
async fn record_refusal(job: &Job, want: Option<&str>, ctx: &Context) {
    let Some(patch) = refusal_patch(job, want) else {
        return;
    };
    let jobs: Api<Job> = Api::namespaced(ctx.client.clone(), &job.namespace().unwrap_or_default());
    if let Err(e) = jobs
        .patch(
            &job.name_any(),
            &kube::api::PatchParams::default(),
            &kube::api::Patch::Merge(patch),
        )
        .await
    {
        tracing::warn!(error = %e, job = %job.name_any(), "could not record the session's refusal");
    }
}

/// Reconcile one projected browse session Job: copy its credentials in when
/// every gate holds, say why on the Job when one does not.
pub async fn reconcile(job: Arc<Job>, ctx: Arc<Context>) -> Result<Action> {
    let start = std::time::Instant::now();
    let result = reconcile_inner(&job, &ctx).await;
    ctx.metrics
        .record_reconcile("BrowseSession", start.elapsed().as_secs_f64());
    result
}

async fn reconcile_inner(job: &Job, ctx: &Context) -> Result<Action> {
    if job_is_done(job) {
        return Ok(Action::await_change());
    }
    let namespace = job.namespace().unwrap_or_default();
    match plan(job, &namespace, ctx).await? {
        Ok((repository, refs)) => {
            match project(job, &namespace, &repository, &refs, ctx).await {
                Ok(()) => record_refusal(job, None, ctx).await,
                // Something the user must change (the operator's Secret RBAC, a
                // missing source): say so on the Job, then retry as usual.
                Err(Error::MissingDependency(why)) => {
                    record_refusal(job, Some(&why), ctx).await;
                    return Err(Error::MissingDependency(why));
                }
                Err(e) => return Err(e),
            }
            Ok(Action::await_change())
        }
        Err(refusal) => {
            let message = refusal.message();
            io::publish_warning_event(
                ctx,
                job,
                "CredentialsNotProjected",
                "ProjectCredentials",
                &message,
            )
            .await;
            record_refusal(job, Some(&message), ctx).await;
            Ok(after_refusal())
        }
    }
}

/// Fetch what the gates read and decide. The outer `Result` is the apiserver;
/// the inner one is the decision.
async fn plan(
    job: &Job,
    namespace: &str,
    ctx: &Context,
) -> Result<Result<(String, Vec<CredsSecretRef>), Refusal>> {
    let Some(snapshot_name) = job.annotations().get(SESSION_SNAPSHOT_ANNOTATION).cloned() else {
        return Ok(Err(Refusal::NoSnapshot));
    };
    let snapshots: Api<Snapshot> = Api::namespaced(ctx.client.clone(), namespace);
    let Some(snapshot) = snapshots.get_opt(&snapshot_name).await? else {
        return Ok(Err(Refusal::SnapshotNotFound {
            snapshot: snapshot_name,
        }));
    };
    let repository = match session_repository(job, &snapshot) {
        Ok(name) => name,
        Err(refusal) => return Ok(Err(refusal)),
    };
    let repos: Api<ClusterRepository> = Api::all(ctx.client.clone());
    let Some(repo) = repos.get_opt(&repository).await? else {
        return Ok(Err(Refusal::RepositoryNotFound { repository }));
    };
    let namespaces: Api<Namespace> = Api::all(ctx.client.clone());
    let labels = namespaces
        .get_opt(namespace)
        .await?
        .map(|n| n.metadata.labels.unwrap_or_default());
    Ok(
        session_gates(job, namespace, &snapshot, &repo, labels.as_ref())
            .map(|refs| (repository, refs)),
    )
}

/// Write the copies, through the same projection every mover's credentials
/// take (ownership guard, source re-read, force apply, 403 hint).
async fn project(
    job: &Job,
    namespace: &str,
    repository: &str,
    refs: &[CredsSecretRef],
    ctx: &Context,
) -> Result<()> {
    let names: Vec<String> = refs.iter().map(|r| r.name.clone()).collect();
    let creds_ctx = CredsContext {
        secret_names: &names,
        repo_kind: "ClusterRepository",
        repo_name: repository,
        repo_secret_namespace: refs.first().and_then(|r| r.namespace.as_deref()),
    };
    let copied = io::resolve_mover_creds(
        &ctx.client,
        namespace,
        &CredsPrefix::browse_session(&job.name_any()),
        &copy_owner(job),
        refs,
        // Both gates were checked above against live objects.
        true,
        true,
        &creds_ctx,
    )
    .await?;
    if copied.projected > 0 {
        ctx.metrics
            .inc_secrets_projected(namespace, copied.projected);
    }
    Ok(())
}

/// Requeue a failed reconcile with the shared backoff and failure Event.
pub fn error_policy(job: Arc<Job>, err: &crate::error::Error, ctx: Arc<Context>) -> Action {
    error_policy_for("BrowseSession", job.as_ref(), err, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO_UID: &str = "crepo-uid";

    fn job(annotations: serde_json::Value, owner_uid: &str) -> Job {
        serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "kopiur-browse-rustfs-0123abcd",
                "namespace": "vaultwarden",
                "uid": "job-uid",
                "annotations": annotations,
                "ownerReferences": [{
                    "apiVersion": "kopiur.home-operations.com/v1alpha1",
                    "kind": "ClusterRepository",
                    "name": "rustfs",
                    "uid": owner_uid,
                    "controller": true
                }]
            }
        }))
        .expect("job fixture")
    }

    fn typed<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> T {
        serde_json::from_value(v).expect("fixture")
    }

    fn snapshot_in(kind: &str, repository: &str, projection: serde_json::Value) -> Snapshot {
        let mut resolved = serde_json::json!({
            "repository": { "kind": kind, "name": repository }
        });
        if !projection.is_null() {
            resolved["credentialProjection"] = projection;
        }
        typed(serde_json::json!({
            "apiVersion": "kopiur.home-operations.com/v1alpha1",
            "kind": "Snapshot",
            "metadata": { "name": "vaultwarden-1", "namespace": "vaultwarden" },
            "spec": {},
            "status": { "resolved": resolved }
        }))
    }

    fn snapshot(projection: serde_json::Value) -> Snapshot {
        snapshot_in("ClusterRepository", "rustfs", projection)
    }

    fn repo(allowed: bool, namespaces: serde_json::Value) -> ClusterRepository {
        let creds = serde_json::json!({ "name": "kopia-rustfs-creds", "namespace": "kopiur" });
        typed(serde_json::json!({
            "apiVersion": "kopiur.home-operations.com/v1alpha1",
            "kind": "ClusterRepository",
            "metadata": { "name": "rustfs", "uid": REPO_UID },
            "spec": {
                "backend": { "s3": { "bucket": "b", "auth": { "secretRef": creds } } },
                "encryption": { "passwordSecretRef": {
                    "name": "kopia-rustfs-creds", "key": "KOPIA_PASSWORD", "namespace": "kopiur"
                } },
                "credentialProjection": { "allowed": allowed },
                "allowedNamespaces": namespaces
            }
        }))
    }

    fn opted_in() -> serde_json::Value {
        serde_json::json!({ "enabled": true })
    }

    fn all() -> serde_json::Value {
        serde_json::json!({ "all": true })
    }

    fn annotated() -> serde_json::Value {
        serde_json::json!({ SESSION_SNAPSHOT_ANNOTATION: "vaultwarden-1" })
    }

    #[test]
    fn a_session_of_an_opted_in_snapshot_gets_the_repositorys_secrets() {
        let refs = session_gates(
            &job(annotated(), REPO_UID),
            "vaultwarden",
            &snapshot(opted_in()),
            &repo(true, all()),
            Some(&BTreeMap::new()),
        )
        .unwrap();
        assert_eq!(
            refs,
            vec![CredsSecretRef {
                name: "kopia-rustfs-creds".into(),
                namespace: Some("kopiur".into()),
            }]
        );
    }

    #[test]
    fn each_closed_gate_refuses_with_its_own_fix() {
        let j = job(annotated(), REPO_UID);
        let labels = BTreeMap::new();
        let gates = |snap: &Snapshot, r: &ClusterRepository, job: &Job| {
            session_gates(job, "vaultwarden", snap, r, Some(&labels)).unwrap_err()
        };
        assert_eq!(
            gates(&snapshot(opted_in()), &repo(false, all()), &j),
            Refusal::OwnerNotAllowed {
                repository: "rustfs".into()
            }
        );
        assert_eq!(
            gates(
                &snapshot(serde_json::json!({ "enabled": false })),
                &repo(true, all()),
                &j
            ),
            Refusal::ConsumerNotOptedIn {
                snapshot: "vaultwarden-1".into()
            }
        );
        assert_eq!(
            gates(&snapshot(serde_json::Value::Null), &repo(true, all()), &j),
            Refusal::ConsumerNotOptedIn {
                snapshot: "vaultwarden-1".into()
            },
            "a snapshot with no pin never opted in"
        );
        assert_eq!(
            gates(
                &snapshot(opted_in()),
                &repo(true, serde_json::json!({ "list": ["media"] })),
                &j
            ),
            Refusal::NamespaceNotAllowed {
                namespace: "vaultwarden".into(),
                repository: "rustfs".into()
            }
        );
        assert_eq!(
            gates(
                &snapshot(opted_in()),
                &repo(true, all()),
                &job(annotated(), "another-uid")
            ),
            Refusal::NotThisRepository {
                repository: "rustfs".into()
            },
            "a same-named repository recreated since is not the one the session holds"
        );
        for refusal in [
            Refusal::OwnerNotAllowed {
                repository: "rustfs".into(),
            },
            Refusal::ConsumerNotOptedIn {
                snapshot: "vaultwarden-1".into(),
            },
        ] {
            assert!(refusal.message().contains("Fix:"), "{}", refusal.message());
        }
    }

    #[test]
    fn the_snapshot_must_live_in_the_repository_that_owns_the_job() {
        let j = job(annotated(), REPO_UID);
        assert_eq!(
            session_repository(&j, &snapshot(opted_in())).unwrap(),
            "rustfs"
        );
        let elsewhere = snapshot_in("ClusterRepository", "other", opted_in());
        assert_eq!(
            session_repository(&j, &elsewhere).unwrap_err(),
            Refusal::NotThisRepository {
                repository: "other".into()
            }
        );
        let namespaced = snapshot_in("Repository", "nas", opted_in());
        assert_eq!(
            session_repository(&j, &namespaced).unwrap_err(),
            Refusal::NotAClusterRepository {
                snapshot: "vaultwarden-1".into()
            }
        );
    }

    #[test]
    fn the_copy_is_owned_by_the_session_job_and_named_as_the_job_asks() {
        let j = job(annotated(), REPO_UID);
        let owner = copy_owner(&j);
        assert_eq!(
            (owner.kind.as_str(), owner.api_version.as_str()),
            ("Job", "batch/v1")
        );
        assert_eq!(owner.uid, "job-uid");
        assert_eq!(owner.controller, Some(true));
        // The longest session Job name kopiur_ops builds is 47 characters; its
        // copies must be named exactly what the Job's envFrom loads.
        let longest = format!("kopiur-browse-{}-0123abcd", "r".repeat(24));
        for idx in 0..=kopiur_api::creds::MAX_CREDS_IDX {
            assert_eq!(
                CredsPrefix::browse_session(&longest).secret_name(idx),
                kopiur_api::creds::browse_session_copy_name(&longest, idx)
            );
        }
    }

    #[test]
    fn the_refusal_on_the_job_is_written_only_when_it_changes() {
        let mut j = job(annotated(), REPO_UID);
        let set = refusal_patch(&j, Some("no")).expect("an absent reason is written");
        assert_eq!(
            set["metadata"]["annotations"][SESSION_CREDS_REFUSED_ANNOTATION],
            "no"
        );
        j.metadata
            .annotations
            .get_or_insert_with(Default::default)
            .insert(SESSION_CREDS_REFUSED_ANNOTATION.into(), "no".into());
        assert_eq!(
            refusal_patch(&j, Some("no")),
            None,
            "the same reason is left alone"
        );
        let cleared = refusal_patch(&j, None).expect("a stale reason is cleared");
        assert!(cleared["metadata"]["annotations"][SESSION_CREDS_REFUSED_ANNOTATION].is_null());
        assert_eq!(
            refusal_patch(&job(annotated(), REPO_UID), None),
            None,
            "nothing to clear"
        );
    }

    #[test]
    fn a_refused_session_is_checked_again_so_fixing_a_gate_lets_it_start() {
        // Nothing the gates read is watched; a refusal re-checks on a timer
        // while the session lives, so allowing it lets the waiting pod start.
        assert_eq!(after_refusal(), Action::requeue(REFUSAL_RECHECK));
        assert!(REFUSAL_RECHECK <= std::time::Duration::from_secs(30));
    }

    #[test]
    fn a_finished_or_deleted_session_needs_nothing() {
        let mut j = job(annotated(), REPO_UID);
        assert!(!job_is_done(&j));
        j.status = Some(
            serde_json::from_value(serde_json::json!({
                "conditions": [{ "type": "Complete", "status": "True" }]
            }))
            .unwrap(),
        );
        assert!(job_is_done(&j));
        let mut deleting = job(annotated(), REPO_UID);
        deleting.metadata.deletion_timestamp =
            Some(serde_json::from_value(serde_json::json!("2026-10-11T12:00:00Z")).unwrap());
        assert!(job_is_done(&deleting));
    }
}
