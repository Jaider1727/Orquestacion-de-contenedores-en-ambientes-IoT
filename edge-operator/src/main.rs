use kube::api::{Api, PostParams, PatchParams, Patch};
use kube::derive::CustomResource;
use kube::runtime::controller::{Controller, Context};
use kube::runtime::reflector::ObjectRef;
use kube::Client;
use serde::{Deserialize, Serialize};
use futures::StreamExt;
use tracing::{info, error};
use k8s_openapi::api::apps::v1::Deployment;
use std::sync::Arc;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug)]
#[kube(group = "edge.example.com", version = "v1", kind = "EdgeDeployment", plural = "edgedeployments", namespaced)]
#[kube(status = "EdgeDeploymentStatus")]
pub struct EdgeDeploymentSpec {
    pub image: String,
    #[serde(default = "default_replicas")]
    pub replicas: i32,
    #[serde(default)]
    pub maxLatencyMs: Option<i32>,
    #[serde(default)]
    pub minBandwidthMbps: Option<i32>,
    #[serde(default)]
    pub nodeSelector: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct EdgeDeploymentStatus {
    pub phase: Option<String>,
    pub reason: Option<String>,
}

fn default_replicas() -> i32 { 1 }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let ed_api: Api<EdgeDeployment> = Api::all(client.clone());

    let controller = Controller::new(ed_api, Default::default())
        .run(reconcile, error_policy, Context::new(Data { client: client.clone() }));

    controller.for_each(|res| async move {
        match res {
            Ok(o) => info!("reconciled {:?}", o),
            Err(e) => error!("reconcile failed: {:?}", e),
        }
    }).await;

    Ok(())
}

struct Data { client: Client }

async fn reconcile(ed: Arc<EdgeDeployment>, ctx: Context<Data>) -> Result<(), anyhow::Error> {
    let client = &ctx.get_ref().client;
    let ns = ed.namespace().unwrap_or_else(|| "default".to_string());
    let name = ed.name_any();

    let mut labels = std::collections::BTreeMap::new();
    labels.insert("app".to_string(), name.clone());

    let replicas = ed.spec.replicas;
    let image = ed.spec.image.clone();

    let mut pod_spec = k8s_openapi::api::core::v1::PodSpec { ..Default::default() };
    let container = k8s_openapi::api::core::v1::Container {
        name: name.clone(),
        image: Some(image.clone()),
        ..Default::default()
    };
    pod_spec.containers = vec![container];

    if let Some(nsmap) = &ed.spec.nodeSelector {
        pod_spec.node_selector = Some(nsmap.clone());
    }

    let deploy = Deployment {
        metadata: kube::core::ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            labels: Some(labels.iter().map(|(k,v)| (k.clone(), v.clone())).collect()),
            ..Default::default()
        },
        spec: Some(k8s_openapi::api::apps::v1::DeploymentSpec {
            replicas: Some(replicas),
            selector: k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: k8s_openapi::api::core::v1::PodTemplateSpec {
                metadata: Some(kube::core::ObjectMeta { labels: Some(labels.clone()), ..Default::default() }),
                spec: Some(pod_spec),
            },
            ..Default::default()
        }),
        status: None,
    };

    let deploys: Api<Deployment> = Api::namespaced(client.clone(), &ns);

    match deploys.patch(&name, &PatchParams::apply("edge-operator"), &Patch::Apply(&deploy)).await {
        Ok(_) => {
            let ed_api: Api<EdgeDeployment> = Api::namespaced(client.clone(), &ns);
            let mut new_status = ed.status.clone().unwrap_or_default();
            new_status.phase = Some("Running".to_string());
            let status_patch = serde_json::json!({"status": new_status});
            let _ = ed_api.patch_status(&name, &PatchParams::default(), &Patch::Merge(&status_patch)).await;
            Ok(())
        }
        Err(e) => {
            let ed_api: Api<EdgeDeployment> = Api::namespaced(client.clone(), &ns);
            let mut new_status = ed.status.clone().unwrap_or_default();
            new_status.phase = Some("Error".to_string());
            new_status.reason = Some(format!("{}", e));
            let status_patch = serde_json::json!({"status": new_status});
            let _ = ed_api.patch_status(&name, &PatchParams::default(), &Patch::Merge(&status_patch)).await;
            Err(anyhow::anyhow!(e))
        }
    }
}

fn error_policy(_object: Arc<EdgeDeployment>, _error: &anyhow::Error, _ctx: Context<Data>) -> kube::runtime::controller::ReconcilerAction {
    kube::runtime::controller::ReconcilerAction { requeue_after: Some(std::time::Duration::from_secs(10)), }
}
