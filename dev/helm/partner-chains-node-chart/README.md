# Setup Local Shell

```
nix-shell -p kubectl awscli2 kubernetes-helm
export AWS_SECRET_ACCESS_KEY=<key>
export AWS_ACCESS_KEY_ID=<key>
aws eks update-kubeconfig --region eu-central-1 --name iog-sidechain-substrate-kubernetes
```

# Helm

## Render Pod: This can be used to check the pod manifest before deploying. This does not change the state of the cluster resources.

```
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/bob.yaml
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/charlie.yaml
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/dave.yaml
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/eve.yaml
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/ferdie.yaml
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/greg.yaml
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/henry.yaml
```

## Upgrading a release if it exists, install if it doesn't. This can be used to update the configuration of any existing resources, and recreate any resources that have been deleted.

```
helm upgrade --install alice . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
helm upgrade --install bob . -f values/chains/devnet.yaml -f values/nodes/devnet/bob.yaml
helm upgrade --install charlie . -f values/chains/devnet.yaml -f values/nodes/devnet/charlie.yaml
helm upgrade --install dave . -f values/chains/devnet.yaml -f values/nodes/devnet/dave.yaml
helm upgrade --install eve . -f values/chains/devnet.yaml -f values/nodes/devnet/eve.yaml
helm upgrade --install ferdie . -f values/chains/devnet.yaml -f values/nodes/devnet/ferdie.yaml
helm upgrade --install greg . -f values/chains/devnet.yaml -f values/nodes/devnet/greg.yaml
helm upgrade --install henry . -f values/chains/devnet.yaml -f values/nodes/devnet/henry.yaml
```

## Deploy Pod: This can be used to deploy a pod if it does not exist.

```
helm install alice . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
helm install bob . -f values/chains/devnet.yaml -f values/nodes/devnet/bob.yaml
helm install charlie . -f values/chains/devnet.yaml -f values/nodes/devnet/charlie.yaml
helm install dave . -f values/chains/devnet.yaml -f values/nodes/devnet/dave.yaml
helm install eve . -f values/chains/devnet.yaml -f values/nodes/devnet/eve.yaml
helm install ferdie . -f values/chains/devnet.yaml -f values/nodes/devnet/ferdie.yaml
helm install greg . -f values/chains/devnet.yaml -f values/nodes/devnet/greg.yaml
helm install henry . -f values/chains/devnet.yaml -f values/nodes/devnet/henry.yaml
```

## Nuke all pods and resources and persistent storage

```
helm uninstall alice
helm uninstall bob
helm uninstall charlie
helm uninstall dave
helm uninstall eve
helm uninstall ferdie
helm uninstall greg
helm uninstall henry
```

# Destroy specific resources:

```
kubectl delete pod <pod> -n sc
kubectl delete pvc <pvc> -n sc
kubectl delete pv <pv> -n sc
kubectl delete svc <svc> -n sc
```

## Listing all deployed releases

```
helm list
```

## Rolling back a release

```
helm rollback alice 1
```

## Viewing the status of a release

```
helm status alice
```

## Packaging the chart

```
helm package .
```

# Kubectl

## Get Queries

```
kubectl get pods -n sc          # high level summary
kubectl get sc -n sc            # storage classes
kubectl get pvc -n sc           # persistent volume claims
kubectl get pv -n sc            # persistent volumes
kubectl get svc -n sc           # services
kubectl get node -n sc          # nodes
kubectl get pods -o wide -n sc  # full summary
```

## Describe Queries

```
kubectl describe pods -n sc          # detailed summary
kubectl describe sc -n sc            # storage classes
kubectl describe pvc -n sc           # persistent volume claims
kubectl describe pv -n sc            # persistent volumes
kubectl describe svc -n sc           # services
kubectl describe node -n sc          # nodes
```

## Logs

```
kubectl logs alice -c cardano-node -n sc
kubectl logs alice -c db-sync -n sc
kubectl logs alice -c postgres -n sc
kubectl logs alice -c substrate-node -n sc

kubectl logs bob -c cardano-node -n sc
kubectl logs bob -c db-sync -n sc
kubectl logs bob -c postgres -n sc
kubectl logs bob -c substrate-node -n sc

kubectl logs charlie -c cardano-node -n sc
kubectl logs charlie -c db-sync -n sc
kubectl logs charlie -c postgres -n sc
kubectl logs charlie -c substrate-node -n sc

kubectl logs dave -c cardano-node -n sc
kubectl logs dave -c db-sync -n sc
kubectl logs dave -c postgres -n sc
kubectl logs dave -c substrate-node -n sc

kubectl logs eve -c cardano-node -n sc
kubectl logs eve -c db-sync -n sc
kubectl logs eve -c postgres -n sc
kubectl logs eve -c substrate-node -n sc

kubectl logs ferdie -c cardano-node -n sc
kubectl logs ferdie -c db-sync -n sc
kubectl logs ferdie -c postgres -n sc
kubectl logs ferdie -c substrate-node -n sc

kubectl logs greg -c cardano-node -n sc
kubectl logs greg -c db-sync -n sc
kubectl logs greg -c postgres -n sc
kubectl logs greg -c substrate-node -n sc
```

## Exec

```
kubectl exec alice -it -c cardano-node -n sc -- sh
kubectl exec alice -it -c db-sync -n sc -- sh
kubectl exec alice -it -c postgres -n sc -- sh
kubectl exec alice -it -c substrate-node -n sc -- sh

kubectl exec bob -it -c cardano-node -n sc -- sh
kubectl exec bob -it -c db-sync -n sc -- sh
kubectl exec bob -it -c postgres -n sc -- sh
kubectl exec bob -it -c substrate-node -n sc -- sh

kubectl exec charlie -it -c cardano-node -n sc -- sh
kubectl exec charlie -it -c db-sync -n sc -- sh
kubectl exec charlie -it -c postgres -n sc -- sh
kubectl exec charlie -it -c substrate-node -n sc -- sh

kubectl exec dave -it -c cardano-node -n sc -- sh
kubectl exec dave -it -c db-sync -n sc -- sh
kubectl exec dave -it -c postgres -n sc -- sh
kubectl exec dave -it -c substrate-node -n sc -- sh

kubectl exec eve -it -c cardano-node -n sc -- sh
kubectl exec eve -it -c db-sync -n sc -- sh
kubectl exec eve -it -c postgres -n sc -- sh
kubectl exec eve -it -c substrate-node -n sc -- sh

kubectl exec ferdie -it -c cardano-node -n sc -- sh
kubectl exec ferdie -it -c db-sync -n sc -- sh
kubectl exec ferdie -it -c postgres -n sc -- sh
kubectl exec ferdie -it -c substrate-node -n sc -- sh

kubectl exec greg -it -c cardano-node -n sc -- sh
kubectl exec greg -it -c db-sync -n sc -- sh
kubectl exec greg -it -c postgres -n sc -- sh
kubectl exec greg -it -c substrate-node -n sc -- sh
```

# Keystores

kubectl create secret generic devnet-alice-keystore --from-file=devnet-alice-keystore/ -n sc
kubectl create secret generic devnet-bob-keystore --from-file=devnet-bob-keystore/ -n sc
kubectl create secret generic devnet-charlie-keystore --from-file=devnet-charlie-keystore/ -n sc
kubectl create secret generic devnet-dave-keystore --from-file=devnet-dave-keystore/ -n sc
kubectl create secret generic devnet-eve-keystore --from-file=devnet-eve-keystore/ -n sc
kubectl create secret generic devnet-ferdie-keystore --from-file=devnet-ferdie-keystore/ -n sc
kubectl create secret generic devnet-greg-keystore --from-file=devnet-greg-keystore/ -n sc
kubectl create secret generic devnet-henry-keystore --from-file=devnet-henry-keystore/ -n sc

kubectl create secret generic staging-validator-1-keystore --from-file=staging-validator-1-keystore/ -n staging
kubectl create secret generic staging-validator-2-keystore --from-file=staging-validator-2-keystore/ -n staging
kubectl create secret generic staging-validator-3-keystore --from-file=staging-validator-3-keystore/ -n staging
kubectl create secret generic staging-validator-4-keystore --from-file=staging-validator-4-keystore/ -n staging
kubectl create secret generic staging-validator-5-keystore --from-file=staging-validator-5-keystore/ -n staging
kubectl create secret generic staging-validator-6-keystore --from-file=staging-validator-6-keystore/ -n staging
kubectl create secret generic staging-validator-7-keystore --from-file=staging-validator-7-keystore/ -n staging
kubectl create secret generic staging-validator-8-keystore --from-file=staging-validator-8-keystore/ -n staging

kubectl create secret generic staging-passive-1-keystore --from-file=staging-passive-1-keystore/ -n staging
kubectl create secret generic staging-passive-2-keystore --from-file=staging-passive-2-keystore/ -n staging
kubectl create secret generic staging-passive-3-keystore --from-file=staging-passive-3-keystore/ -n staging
kubectl create secret generic staging-passive-4-keystore --from-file=staging-passive-4-keystore/ -n staging
kubectl create secret generic staging-passive-5-keystore --from-file=staging-passive-5-keystore/ -n staging
kubectl create secret generic staging-passive-6-keystore --from-file=staging-passive-6-keystore/ -n staging
kubectl create secret generic staging-passive-7-keystore --from-file=staging-passive-7-keystore/ -n staging
kubectl create secret generic staging-passive-8-keystore --from-file=staging-passive-8-keystore/ -n staging

# Scenarios

## Scenario 1: Make permanent changes to pod manfiest and deploy without wiping persistent storage

### 1.1. Make changes to pod manifest in templates directory. For certain changes it may be appropriate to introduce new values in the values files and reference them in the pod manifest. Ensure consitency between the values files.

```
substrate-node-stack-chart/templates/
substrate-node-stack-chart/values/
```

### 1.2. Delete the pod:

```
kubectl delete pod alice -n sc
```

### 1.3. Deploy the pod:

```
helm upgrade alice . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
```

## Scenario 2: Make temporary changes to resources

### 2.1. Render the pod manifest to a file:

```
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml > /tmp/alice-modified.yaml
```

### 2.2. Make changes to the pod manifest in the file:

```
vim /tmp/alice-modified.yaml
```

### 2.3. Delete neccessary resources:

```
kubectl delete pod alice -n sc
```

### 2.4. Apply the modified pod manifest:

```
kubectl apply -f /tmp/alice-modified.yaml
```

## Scenario 3: Destroy node and wipe persistent storage

### 3.1. Delete the pod:

```
helm uninstall alice
```

### 3.2. Reinstate the pod:

```
helm install alice . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
```

## Scenario 4: Destroy all nodes without wiping persistent storage

### 4.1. Delete all pods:

```
kubectl delete pod alice -n sc
kubectl delete pod bob -n sc
kubectl delete pod charlie -n sc
kubectl delete pod dave -n sc
kubectl delete pod eve -n sc
kubectl delete pod ferdie -n sc
kubectl delete pod greg -n sc
kubectl delete pod henry -n sc
```

### 4.2. Reinstate all pods:

```
helm install alice . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
helm install bob . -f values/chains/devnet.yaml -f values/nodes/devnet/bob.yaml
helm install charlie . -f values/chains/devnet.yaml -f values/nodes/devnet/charlie.yaml
helm install dave . -f values/chains/devnet.yaml -f values/nodes/devnet/dave.yaml
helm install eve . -f values/chains/devnet.yaml -f values/nodes/devnet/eve.yaml
helm install ferdie . -f values/chains/devnet.yaml -f values/nodes/devnet/ferdie.yaml
helm install greg . -f values/chains/devnet.yaml -f values/nodes/devnet/greg.yaml
helm install henry . -f values/chains/devnet.yaml -f values/nodes/devnet/henry.yaml
```

## Scenario 5: Destroy all nodes and wipe persistent storage

### 5.1. Delete all pods and resources:

```
helm uninstall alice
helm uninstall bob
helm uninstall charlie
helm uninstall dave
helm uninstall eve
helm uninstall ferdie
helm uninstall greg
helm uninstall henry
```

### 5.2. Reinstate all pods:

```
helm install alice . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
helm install bob . -f values/chains/devnet.yaml -f values/nodes/devnet/bob.yaml
helm install charlie . -f values/chains/devnet.yaml -f values/nodes/devnet/charlie.yaml
helm install dave . -f values/chains/devnet.yaml -f values/nodes/devnet/dave.yaml
helm install eve . -f values/chains/devnet.yaml -f values/nodes/devnet/eve.yaml
helm install ferdie . -f values/chains/devnet.yaml -f values/nodes/devnet/ferdie.yaml
helm install greg . -f values/chains/devnet.yaml -f values/nodes/devnet/greg.yaml
helm install henry . -f values/chains/devnet.yaml -f values/nodes/devnet/henry.yaml
```

## Scenario 6: Access exposed services from outside the cluster (e.g. from a local machine):

### 6.1. Get the port of the service:

```
kubectl get svc alice -n sc
```

### 6.2. Forward the port:

```
kubectl port-forward svc/alice 8080:8080 -n sc
```

### 6.3. Access the service:

```
curl localhost:8080
```

## Scenario 7: Check disk usage of a pod:

### 7.1. Use below commands with the appropriate pod name:

```
kubectl exec alice -it -c cardano-node -n sc -- sh -c "cd /data && du -sh"
kubectl exec alice -it -c db-sync -n sc -- sh -c "cd /var/lib && du -sh"
kubectl exec alice -it -c postgres -n sc -- sh -c "cd /var/lib/postgresql/data && du -sh"
kubectl exec alice -it -c substrate-node -n sc -- sh -c "cd /data && du -sh"
```

### Scenario 8: Wipe Substrate Node Persistent Storage

### 8.1 Delete pods:

devnet:

```
kubectl delete pod alice -n sc --wait=false
kubectl delete pod bob -n sc --wait=false
kubectl delete pod charlie -n sc --wait=false
kubectl delete pod dave -n sc --wait=false
kubectl delete pod eve -n sc --wait=false
kubectl delete pod ferdie -n sc --wait=false
kubectl delete pod greg -n sc --wait=false
kubectl delete pod henry -n sc --wait=false
```

staging-preview:

```
for i in {1..4}; do kubectl delete pod staging-preview-validator-$i -n staging-preview --wait=false; done
```

### 8.2. Delete substrate-node PVCs

devnet:

```
kubectl delete pvc alice-claim-substrate-node-data -n sc
kubectl delete pvc bob-claim-substrate-node-data -n sc
kubectl delete pvc charlie-claim-substrate-node-data -n sc
kubectl delete pvc dave-claim-substrate-node-data -n sc
kubectl delete pvc eve-claim-substrate-node-data -n sc
kubectl delete pvc ferdie-claim-substrate-node-data -n sc
kubectl delete pvc greg-claim-substrate-node-data -n sc
kubectl delete pvc henry-claim-substrate-node-data -n sc
```

staging-preview:

```
for i in {1..4}; do kubectl delete pvc staging-preview-validator-$i-claim-substrate-node-data -n staging-preview; done
```

### 8.3. Reinstate all pods:

```
cd src/kube/substrate-poc/environments/helm/substrate-node-stack-chart
```

devnet:

```
helm upgrade --install alice . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
helm upgrade --install bob . -f values/chains/devnet.yaml -f values/nodes/devnet/bob.yaml
helm upgrade --install charlie . -f values/chains/devnet.yaml -f values/nodes/devnet/charlie.yaml
helm upgrade --install dave . -f values/chains/devnet.yaml -f values/nodes/devnet/dave.yaml
helm upgrade --install eve . -f values/chains/devnet.yaml -f values/nodes/devnet/eve.yaml
helm upgrade --install ferdie . -f values/chains/devnet.yaml -f values/nodes/devnet/ferdie.yaml
helm upgrade --install greg . -f values/chains/devnet.yaml -f values/nodes/devnet/greg.yaml
helm upgrade --install henry . -f values/chains/devnet.yaml -f values/nodes/devnet/henry.yaml
```

staging-preview:

```
for i in {1..4}; do helm upgrade --install staging-preview-validator-$i . -f values/chains/staging-preview.yaml -f values/nodes/staging-preview/validator/staging-preview-validator-$i; done
```

### Scenario 9: Deploy with --chain-spec arg

### 9.1 Upload local chain-spec file as secret:

```
kubectl create secret generic <secretname> --from-file=<filename> --namespace=<namespace>
```

### 9.2 Enable chain-spec, and provide chain-spec filename, secretName in chain values under values/chains/

```
chain:
  chainspec: true
  chainspec_secretName: "secret123"
  chainspec_filename: "chain-spec-123.json"
```

### 9.3 Verify deployment renders correctly

```
helm template . -f values/chains/devnet.yaml -f values/nodes/devnet/alice.yaml
```

### 9.4 Delete pod and follow usual deployment process
