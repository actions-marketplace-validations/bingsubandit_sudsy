#!/usr/bin/env bash
# Fetches the pinned projects of the Ansible evidence (docs/ai/design/ansible.md) into
# tests/conformance/corpus/ansible/<repo> (gitignored): every YAML file of 57 public projects (collections, roles, playbook
# repositories), chosen before any result was seen; `scripts/evidence_ansible.py` recognises the Ansible files and caps a project.
export TARBALL_LENIENT=1 TARBALL_OWNER=1
source "$(dirname "$0")/tarball-corpus.sh"
P='*.yml *.yaml !*/.github/* !*/node_modules/* !*/vendor/* !*/changelogs/* !*/.git/*'
for line in \
  'ansible-collections/community.general 4d51e8dc62be3841710ca82ab079890a542a5faf $P' \
  'ansible-collections/community.docker daf48a88d6de33276fdf51f99e4a208ec61c36d1 $P' \
  'ansible-collections/community.crypto 5817063a1d08ce4a00377315d0891f87783d66fb $P' \
  'ansible-collections/ansible.posix 4daa61d2948b35f828ea9e0fa261ca13cb87713a $P' \
  'ansible-collections/ansible.utils fa9823a51c1257860018ad429ef013047a756a46 $P' \
  'ansible-collections/community.mysql 4cf7a3176fbd07be43459f1366e8066381df6d97 $P' \
  'ansible-collections/community.postgresql 32b273d9c30a4d66fc29a5b3812f7e98ead13a2e $P' \
  'ansible-collections/kubernetes.core 44c9359c5cbfcb9caec326882939eb803f55d170 $P' \
  'ansible-collections/amazon.aws f2a68f5e39fcb949c1f769c556b634c437a5b05e $P' \
  'ansible-collections/ansible.netcommon 681cee698cd227bdf509311d0a9e582702a25b77 $P' \
  'ansible-collections/cisco.ios f8a0cc5d9723cfd4128ce06ef6f6171f1c86c24f $P' \
  'ansible-collections/community.windows 1f10afc4cec8db01601035478147fbec3ce542ec $P' \
  'ansible-collections/ansible.windows cbefbbd7d1f9bf1b244a0c24f29d3d4efc880651 $P' \
  'ansible-collections/community.hashi_vault 2a0b75ce88e1deb08822b587461f2987b7710c72 $P' \
  'ansible-collections/community.vmware 803a0dea259d55613fb4f51055cb61319ab44bc8 $P' \
  'ansible-collections/community.zabbix 9ac1184d77d1dafbbf67964314ff15e4c0876ca9 $P' \
  'theforeman/foreman-ansible-modules 54773dc9e7860fdfba975bfbb3a031451dfafda3 $P' \
  'geerlingguy/ansible-role-nginx 5ff0b235006390a0d5666fd4cce7477410982cdf $P' \
  'geerlingguy/ansible-role-apache e81915e791c3c9d2ccf3d999454d600f6148d0d9 $P' \
  'geerlingguy/ansible-role-mysql 0a0ea6b728120b3ab3918332d9404bb65836834d $P' \
  'geerlingguy/ansible-role-docker 5daab495728fb2b6fa35613039bd26b4034cf414 $P' \
  'geerlingguy/ansible-role-postgresql 3e6b4e683f37fa519b3457a2b8c4d2df06620c3e $P' \
  'geerlingguy/ansible-role-redis 9e8c2faa4ed44add3860c117e708d9cbada99ff5 $P' \
  'geerlingguy/ansible-role-php a7e6e363d3541726458494335f5a8087e3647f98 $P' \
  'geerlingguy/ansible-role-java 8477f288afcce7d4b230ebdf250bf337d0c4ffcd $P' \
  'geerlingguy/ansible-role-security 7a1fe0ef8d1c5fc3c15384a1b503734d31518acd $P' \
  'geerlingguy/ansible-role-haproxy 8b23ff0ffb56ce466eb3a7f3fec995f6f2e91add $P' \
  'geerlingguy/ansible-role-jenkins 5c58839e261a56661011a8efb55277c9d71c372f $P' \
  'geerlingguy/ansible-role-certbot 8180fe8c1aeefe8fcf9760cb94e4ef793b72a935 $P' \
  'geerlingguy/ansible-for-devops 86e1bf80098239cd995d54217585e8d61117d2e7 $P' \
  'robertdebock/ansible-role-docker 034a551bbf83a053808252158149b9fad12f1bee $P' \
  'robertdebock/ansible-role-java 428ab5fa432881d43c78b81d7ee2c56cf74a68d8 $P' \
  'debops/debops 8203286a703654f3df4e59274c765536256ebc0c $P' \
  'ansible-lockdown/RHEL9-CIS 28bdebf766f9785ffef0cd8643f744aa1b752039 $P' \
  'ansible-lockdown/UBUNTU22-CIS acd5b83fabac4970084017d520bcb069912adaf6 $P' \
  'dev-sec/ansible-collection-hardening 40a1348eaf62dd34312df74440d577fcec292cf2 $P' \
  'openstack/kolla-ansible e9d684160da5ea58c5db001946a57fab9e600bd5 $P' \
  'kubernetes-sigs/kubespray 60d8b95d8c3da7760672da86eb48f1d4b4bdd4a7 $P' \
  'ansible/ansible-examples b50586543c6c4be907fdc88f9f78a2b35d2a895f $P' \
  'ansible/workshops 83f99c830f40f6b0953f11689bf825da59183917 $P' \
  'trailofbits/algo 216fab3c4fe7a8e8180e3f495b25caa9ceb3e27f $P' \
  'redhat-cop/infra.aap_configuration 1f0ddd873c81ddaa23db7c34d4f6a2c256dffe9e $P' \
  'ceph/ceph-ansible cf6233d64f31843fc78fa0e349f98ad9b54a24d6 $P' \
  'openstack/openstack-ansible b89e6c423b20941d7002e175e8861770c6b6be01 $P' \
  'mrlesmithjr/ansible-mariadb-galera-cluster 745b836a6d0232a2aa2fb4c4717f4420b74d18cf $P' \
  'ansible/awx 0717600bb067dc817cacaad4e5add83199528764 $P' \
  'ansible/ansible cd313f0a8650ded36707a24e2bad61375f1214ce $P' \
  'nginxinc/ansible-role-nginx 157e0e97406f798bd6f50db37430a78c4269aa92 $P' \
  'ansible-community/molecule 5430d28a50a91af39b761ebcb05bdc40a218eb08 $P' \
  'ansible/ansible-navigator cbe9a4a686d0814177ab9360f00bc3f20b644e3d $P' \
  'freeipa/ansible-freeipa c6f0e6b969b87fa5d5a4bf66da56150fdee6f9b3 $P' \
  'oVirt/ovirt-ansible-collection 094944d4d995655fd928dbf39e77b747657cc0b9 $P' \
  'prometheus-community/ansible 7131b506d9d76b7ed5cb1438a9c6143cfd481929 $P' \
  'cloudalchemy/ansible-prometheus e4393788ac1d8e6608777c9edb6b711b933d9c39 $P' \
  'ansible-collections/community.mongodb 395a0c27e4e19d53d4d12c2ca4e2fc7902ae02f8 $P' \
  'ansible-collections/community.rabbitmq 4c5ceeee342ebd931128e62312833393eb4fd599 $P' \
  'ansible-collections/community.grafana d7ae4090f2da6b13d5462628b780376eb81ddffd $P' \
  ; do
  fetch_tarballs ansible <<<"${line//\$P/$P}" || echo "no YAML: ${line%% *}"
done
