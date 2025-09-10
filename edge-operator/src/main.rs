use kube::{Api, Client, CustomResource, ResourceExt};
use kube::runtime::controller::{Controller, Action};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use futures::StreamExt;
use tracing::{info, error};
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{PodSpec, Container, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ObjectMeta, LabelSelector};
use std::collections::BTreeMap;
use std::sync::Arc;
use anyhow::Result;
use tokio;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "edge.example.com", version = "v1", kind = "EdgeDeployment", plural = "edgedeployments", namespaced)]
#[kube(status = "EdgeDeploymentStatus")]
pub struct EdgeDeploymentSpec {
    pub image: String,
    #[serde(default = "default_replicas")]
    pub replicas: i32,
    #[serde(default)]
    pub max_latency_ms: Option<i32>,
    #[serde(default)]
    pub min_bandwidth_mbps: Option<i32>,
    #[serde(default)]
    pub node_selector: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct EdgeDeploymentStatus {
    pub phase: Option<String>,
    pub reason: Option<String>,
}

fn default_replicas() -> i32 { 1 }

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let ed_api: Api<EdgeDeployment> = Api::all(client.clone());

    Controller::new(ed_api, Default::default())
        .run(reconcile, error_policy, Arc::new(Data { client: client.clone() }))
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled: {:?}", o),
                Err(e) => error!("reconcile failed: {:?}", e),
            }
        })
        .await;

    Ok(())
}

struct Data { client: Client }

async fn reconcile(ed: Arc<EdgeDeployment>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    let client = &ctx.client;
    let ns = ed.namespace().unwrap_or_else(|| "default".into());
    let name = ed.name_any();

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), name.clone());

    let replicas = ed.spec.replicas;
    let image = ed.spec.image.clone();

    let pod_spec = PodSpec {
        containers: vec![Container {
            name: name.clone(),
            image: Some(image.clone()),
            ..Default::default()
        }],
        node_selector: ed.spec.node_selector.clone(),
        ..Default::default()
    };

    let deploy = Deployment {
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            labels: Some(labels.clone()),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(replicas),
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta { labels: Some(labels.clone()), ..Default::default() }),
                spec: Some(pod_spec),
            },
            ..Default::default()
        }),
        status: None,
    };

    let deploys: Api<Deployment> = Api::namespaced(client.clone(), &ns);

    match deploys.replace(&name, &Default::default(), &deploy).await {
        Ok(_) => Ok(Action::requeue(std::time::Duration::from_secs(30))),
        Err(e) => {
            error!("Failed to apply Deployment: {}", e);
            Ok(Action::requeue(std::time::Duration::from_secs(10)))
        }
    }
}

fn error_policy(_object: Arc<EdgeDeployment>, _error: &kube::Error, _ctx: Arc<Data>) -> Action {
    Action::requeue(std::time::Duration::from_secs(10))
}
