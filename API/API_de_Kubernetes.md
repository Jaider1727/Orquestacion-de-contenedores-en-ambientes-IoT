# Conceptos básicos para la extensión de kubernetes

---

## 1 CRD o CustomResourceDefinition 

Un CRD extiende la API de Kubernetes: que permite definir nuevos tipos de recursos (como Pod, Deployment, pero personalizados). Este CRD en el clúster es llamado EdgeDeployment con el cual podemos crear objetos EdgeDeployment que representan las intenciones de despliegue en el borde **egde-crd.yaml**.

## 2 Objetos tipo EdgeDeployment 

Esto es lo que el usuario &/o administrador crearía para desplegar un pod **test-ed.yaml**. 

## 3 Permisos 

Antes de montar el operator y el agent estos necesitan permisos para leer/patch nodes y para poder crear Deployments **rbac-edge**.

---

```bash

## Colocar la ruta de los yaml

kubectl apply -f edge-crd.yaml 
kubectl apply -f test-ed.yaml
kubectl apply -f rbac-edge.yaml

## Comprobar los archivos de configuración

kubectl get serviceaccount -n default | grep edge
kubectl get clusterrole | grep edge
kubectl get clusterrolebinding | grep edge

## Revisión de permisos
## El agente debe poder parchear nodos y el operador debe poder patchear el status del crd "Yes".

sudo kubectl auth can-i patch nodes --as=system:serviceaccount:default:edge-agent
sudo kubectl auth can-i patch edgedeployments/status --as=system:serviceaccount:default:edge-operator --namespace=default

## Comprobaciones utiles
sudo kubectl get sa -n default
sudo kubectl get clusterrole edge-operator-role -o yaml

```