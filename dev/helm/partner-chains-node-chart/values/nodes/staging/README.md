helm install passive-1 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-1
helm install passive-2 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-2
helm install passive-3 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-3
helm install passive-4 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-4
helm install passive-5 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-5
helm install passive-6 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-6
helm install passive-7 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-7
helm install passive-8 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-8

helm install validator-1 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-1
helm install validator-2 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-2
helm install validator-3 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-3
helm install validator-4 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-4
helm install validator-5 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-5
helm install validator-6 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-6
helm install validator-7 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-7

helm upgrade --install passive-1 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-1
helm upgrade --install passive-2 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-2
helm upgrade --install passive-3 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-3
helm upgrade --install passive-4 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-4
helm upgrade --install passive-5 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-5
helm upgrade --install passive-6 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-6
helm upgrade --install passive-7 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-7
helm upgrade --install passive-8 . -f values/chains/staging.yaml -f values/nodes/staging/passive/passive-8

helm upgrade --install validator-1 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-1
helm upgrade --install validator-2 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-2
helm upgrade --install validator-3 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-3
helm upgrade --install validator-4 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-4
helm upgrade --install validator-5 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-5
helm upgrade --install validator-6 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-6
helm upgrade --install validator-7 . -f values/chains/staging.yaml -f values/nodes/staging/validator/validator-7

helm delete passive-1
helm delete passive-2
helm delete passive-3
helm delete passive-4
helm delete passive-5
helm delete passive-6
helm delete passive-7
helm delete passive-8

helm delete validator-1
helm delete validator-2
helm delete validator-3
helm delete validator-4
helm delete validator-5
helm delete validator-6
helm delete validator-7

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
