#!/usr/bin/env bash
# Fetches the pinned projects of the Kubernetes-manifest evidence into tests/conformance/corpus/kubernetes/<owner>__<repo>
# (gitignored): every YAML file of 81 public projects (upstream examples and operators, GitOps homelabs, manifest
# repositories), pinned to the default-branch head resolved on 2026-11-26 and chosen before any result was seen. Helm charts and
# templates are excluded (`!*/charts/*`, `!*/templates/*`).
export TARBALL_LENIENT=1 TARBALL_OWNER=1
source "$(dirname "$0")/tarball-corpus.sh"
P='*.yml *.yaml !*/.github/* !*/node_modules/* !*/vendor/* !*/.git/* !*/charts/* !*/templates/*'
for line in \
  'kubernetes/examples 22934a040acbb0c0aa103023ccbd871507bf17f6 $P' \
  'kubernetes/website 7c81c4a67b17aa329f277a53b9e9111fb03b0ede $P' \
  'GoogleCloudPlatform/kubernetes-engine-samples 2c7091f9066e82d036e16f0a0ab68a18b09d29e1 $P' \
  'GoogleCloudPlatform/microservices-demo c7cb6d5bfd6ea3aed768634ebbcb98cb7a523712 $P' \
  'open-telemetry/opentelemetry-demo a49137190e4c8b01f61b31007bbbd373aa43d606 $P' \
  'argoproj/argocd-example-apps 8088f4c0d970abb09e250248cc97e35623447cb5 $P' \
  'argoproj/argo-workflows 27111e49f2107d1b6e2a7a55c42fad5899e723e5 $P' \
  'argoproj/argo-rollouts fe07313a774f2726648f2de010aca716569f0434 $P' \
  'argoproj/argo-cd 031f847079ccf91453ac98390e75b84f8b44584b $P' \
  'istio/istio 5a221077dc16e40052db6e33ca1c799fb21ef58c $P' \
  'knative/serving fac25dca9ed8dc9178d6f92d7315602c83c1d927 $P' \
  'kubernetes-sigs/kustomize 046e4a1610746b2c8223985b1190463f56c9f2f7 $P' \
  'kubernetes-sigs/metrics-server acd676e54f96285dd4e14707bcec54a726401b74 $P' \
  'kubernetes/dashboard 0ba796d9b85f7070dbd4a5c88354fc79f88763bb $P' \
  'kubernetes/ingress-nginx dbb11b92ddb77bee9c35e462479129354c84f939 $P' \
  'metallb/metallb 0db4a44c098f71b6c18e2be34e73e67d8a39c20b $P' \
  'prometheus-operator/kube-prometheus eb634f11cb010586b5a79f3c10c4ceedd4494696 $P' \
  'kubernetes/kube-state-metrics 46cd52ff39af128e15e64ffd8ba8c75eb6f5a213 $P' \
  'kubernetes-sigs/external-dns ab6a24cc51de597d3d221d10d6d7e00382f47bd8 $P' \
  'rook/rook 394e6dba67087967788c4f2fa9bf719f13427fe4 $P' \
  'cilium/cilium f2e66637e0ccd101c9966740425df950a26ab050 $P' \
  'kubernetes-sigs/gateway-api 6637ebacd65b5ee6822460578c818ffffadb3d62 $P' \
  'strimzi/strimzi-kafka-operator 58722088ee747afc1ca1c54aead2f23c45d276ff $P' \
  'zalando/postgres-operator 469ee913e5e0af91d68ca4bc4ba592c789f25931 $P' \
  'cloudnative-pg/cloudnative-pg b6f21de70cacc888baffdd378aae442a8f6a0d93 $P' \
  'Azure-Samples/aks-store-demo 1d61b202d1e6dc9a78389c15e4bb9d529e5499d3 $P' \
  'dockersamples/example-voting-app 63e9150ca17af4ed05880d4245e486481f73fcb4 $P' \
  'kubernetes-sigs/kind 688af97dff2a4d048973cabc737fba4592d6ccf5 $P' \
  'kubernetes-sigs/cluster-api a992e12a80c5da6d304f30ac3e02107aa1e09083 $P' \
  'kubernetes-sigs/kueue 2879d9f0be9a3974f2b56ecb2495275163ef15bb $P' \
  'kubernetes-sigs/descheduler 7609212587c1c1d0c1f71de979ff8fbfcd896bbb $P' \
  'tektoncd/pipeline 6834c1e939395db8974fbdfb253d7966d1fdb5f0 $P' \
  'tektoncd/catalog 112374ce1262b133a77300bd8133445722605e04 $P' \
  'fluxcd/flux2-kustomize-helm-example 4f1a5b1051b469108d1bdf0ebec6c814427e3834 $P' \
  'fluxcd/flux2-multi-tenancy 16752eb2929feb1d0e7de6ad4fa0a2e1972825ac $P' \
  'onedr0p/home-ops e963d38167f0f34cc468fd6096108dcffdceaad1 $P' \
  'khuedoan/homelab 7485d8201f65e852046bc68c7790957d7da59113 $P' \
  'szinn/k8s-homelab 6bcc7b529e18fd446629f19a12fafaced5922bb2 $P' \
  'bjw-s-labs/home-ops 086c840e0a7ebe5022fdef59208e89fec217cb7c $P' \
  'buroa/k8s-gitops 2c3aad4e592708ffd8080dce56304542dafca315 $P' \
  'mchestr/home-cluster 8efa40d27272b3980f290f3b77e65f26843a0466 $P' \
  'xunholy/k8s-gitops 49a0fe5f33d148a2700e46658f119e82d64fd560 $P' \
  'angelnu/k8s-gitops e0c424b85980af1450515577224d7ea16da0951f $P' \
  'JJGadgets/Biohazard 6d640eef77b1e3d52c9c5822f8a95e9c2ec0ad3e $P' \
  'toboshii/home-ops b16119e828316df78f73d378bf7c386bad9dac12 $P' \
  'kubernetes-sigs/kubebuilder 00c0c90e4fa9dce2d50f451f927a5d8a5237d055 $P' \
  'operator-framework/operator-sdk 299157a814e674f8f40d1b91ee77d64a90e850de $P' \
  'vmware-tanzu/velero 64d40dd74b387cb2029e4486d96ac1a5c568b050 $P' \
  'longhorn/longhorn 5ca10a5a40c6e1cd97ddbcfcfbd7843d84d79e8a $P' \
  'openebs/openebs 4e98dab16c22eb865174d0b221d74217b240d20e $P' \
  'minio/operator 34dc3980190390f4ed2f4b4355dbcb936f667a10 $P' \
  'jaegertracing/jaeger-operator d9ec1714c6014990ad5862a6dc97284e0b23ebf3 $P' \
  'nginxinc/kubernetes-ingress 2f46a26c8b85b90054610f9eb6ab00e3c8125676 $P' \
  'haproxytech/kubernetes-ingress 3798f74fbac3d4d1c00ac7979b3b478340e9fbd4 $P' \
  'projectcalico/calico 8570f9dce2cc3b9f6b472105ebced6f5a4e0184b $P' \
  'flannel-io/flannel 87f7078166efaa224a92da5aae254ee9649729a5 $P' \
  'kubernetes-sigs/aws-load-balancer-controller faa9985480802ca0aef01bb09dce8b5b7a7b553e $P' \
  'kubernetes-sigs/secrets-store-csi-driver e3d1a534319cc8d1750cffda1dfeae15f91ac09f $P' \
  'external-secrets/external-secrets 0055f8ca0e678c73f9b314ba5b5afe673a2d0004 $P' \
  'kyverno/policies cbd2d6991976a512014d43817fa64d6c8b98a7d7 $P' \
  'open-policy-agent/gatekeeper a2bac802bc01a8a1a7639d7b52050ffa79fc785d $P' \
  'hashicorp/vault-k8s 7753d2edaaff02a31ba2c31a4bfc96d45ba5be3b $P' \
  'kubernetes-sigs/cluster-api-provider-aws 672c5b2e3ecdd3f8f27890b28a1fc9ce632014f9 $P' \
  'kubernetes-sigs/kwok 48b0bf5dca9967f26772eb3ddd380571fca2459a $P' \
  'grafana/loki 640be06cb2e2e1830e9424c427ac279e693ac3a5 $P' \
  'prometheus-operator/prometheus-operator 9ead22a323c3a61ea2cd0dd0f2fae0bab6afc928 $P' \
  'kubeflow/manifests d755abccfbd85e5430f18319a6ba7cb318d8f891 $P' \
  'GoogleContainerTools/skaffold 9504a7c4464a384c677130081fd1f56c57e6c37f $P' \
  'tilt-dev/tilt 5f4a7cba3dedf74ff26a79631cf9f380f12186c6 $P' \
  'kubernetes/minikube 61e410233d20e551ff1ec58895ef3d09051300ad $P' \
  'k3s-io/k3s 9b575d910f0ac0052a199f8b86e0c6ccc7c22e47 $P' \
  'rancher/rancher cef389e1ae542978569ce9d44e75aa0b77a1c168 $P' \
  'kubeedge/kubeedge 8a61b67f631bf2059a7d59844d136f3baf87c5c6 $P' \
  'ray-project/kuberay 9bb2d3c80d612d970d5d14f354ca807d2351cdd2 $P' \
  'kserve/kserve 1cee2dbd8d86d32bf41a0da0c1ae70679567d71f $P' \
  'apache/airflow 0354004df00854402e75301d75fba0bcb1f25907 $P' \
  'getsentry/self-hosted cdf152d29766d266c93580e055261bd6589f1fa2 $P' \
  'mastodon/mastodon 6c29cb4b9c1f8b3f9a1be23f0efb9c5ac64155a3 $P' \
  'nextcloud/helm 3d92e1448018dbbc3f6a295c7458b1dc082b7857 $P' \
  'aws-samples/eks-workshop-v2 26aa7eca6e6b24abf5604fea5b7c108d8d995f6e $P' \
  'killercoda/scenarios-kubernetes 7aa5b409ef6fe257e45eaf1671388d1c04cf15d3 $P' \
  ; do
  fetch_tarballs kubernetes <<<"${line//\$P/$P}" || echo "no YAML: ${line%% *}"
done
