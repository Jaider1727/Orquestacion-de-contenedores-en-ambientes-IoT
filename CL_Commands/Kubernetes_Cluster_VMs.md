# Montar un Clúster Kubernetes con K3s

## Requisitos

- Recomendable **tres máquina Linux** (Ubuntu/Debian) para el maestro y para los workers.
- Conectividad de red entre todas las máquinas, con base en donde esten desplegadas las VMs revisar los adaptadores NAT y Host-Only(Solo-anfitrión).
- Acceso con permisos `sudo` o root.
- Tener en cuenta los puertos **6443** para que los nodos worker puedan comunicarse con el API server del maestro.

---

## 1. Nodo Maestro 

Este nodo ejecuta el API Server, Scheduler y componentes del control plane.

```bash
# 1. Actualizar el sistema
sudo apt update && sudo apt upgrade -y

# 2. Instalar K3s (server + utilidades como kubectl)
curl -sfL https://get.k3s.io | sh -

# 3. Verificar que K3s esté activo
kubectl get nodes

# 4. Obtener token para que los workers puedan unirse
sudo cat /var/lib/rancher/k3s/server/node-token

# 5. Obtener la IP accesible del maestro (Buscar la Ip pública)
ip a

```

---

## 2. Nodos Worker 

```bash

# 5. En estos nodos se ejecutarán los pods.
# Este comando instala K3s en modo agente y lo conecta al nodo maestro usando la IP y token obtenido en el paso # 4.
curl -sfL https://get.k3s.io | K3S_URL=https://[IP_DEL_MAESTRO]:6443 K3S_TOKEN=[TOKEN] sh -``

```

---

## 3. Verificación del Clúster

```bash
# 6. Ejecutamos desde el nodo Maestro y deberiamos ver tanto el maestro como los workers en estado Ready.
kubectl get nodes -o wide
kubectl get pods -A

# 7. Si vemos algún error comprobar primero conectividad entre VMs (ping entre VMs), por otro lado si se conecto solo un worker y el otro no, revisar que los hostname no sean iguales, el nodo maestro no admite dos nodos con el mismo nombre.
```

