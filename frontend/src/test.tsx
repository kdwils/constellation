import type { ResourceNode } from "./ResourceNode";

export const testResourceNodes: ResourceNode[] = [
    {
        kind: "Namespace",
        name: "pocketid",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "pocketid",
                relatives: [
                    {
                        kind: "Service",
                        name: "pocketid",
                        relatives: [
                            { kind: "Pod", name: "pocketid-797fb4cd57-nrv7n", relatives: [] },
                        ],
                    },
                ],
            },
        ],
    },
    {
        kind: "Namespace",
        name: "cert-manager",
        relatives: [
            { kind: "Service", name: "cert-manager-webhook", relatives: [] },
            { kind: "Service", name: "cert-manager", relatives: [] },
            { kind: "Service", name: "cert-manager-cainjector", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "rss",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "freshrss",
                relatives: [
                    {
                        kind: "Service",
                        name: "freshrss",
                        relatives: [
                            { kind: "Pod", name: "freshrss-deployment-5bf7d5fd55-7z4dj", relatives: [] },
                        ],
                    },
                ],
            },
        ],
    },
    {
        kind: "Namespace",
        name: "system-upgrade",
        relatives: [
            { kind: "Pod", name: "system-upgrade-controller-8cc947d-nzrjp", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "blog-dev",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "blog-dev",
                relatives: [
                    {
                        kind: "Service",
                        name: "blog",
                        relatives: [
                            { kind: "Pod", name: "blog-76656984d7-k9rgh", relatives: [] },
                        ],
                    },
                ],
            },
        ],
    },
    {
        kind: "Namespace",
        name: "default",
        relatives: [
            { kind: "Service", name: "kubernetes", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "minio",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "minio",
                relatives: [
                    {
                        kind: "Service",
                        name: "minio-console",
                        relatives: [
                            { kind: "Pod", name: "minio-console-68647f7c6-j5dhw", relatives: [] },
                        ],
                    },
                ],
            },
            { kind: "Service", name: "minio", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "vaultwarden",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "vaultwarden",
                relatives: [
                    {
                        kind: "Service",
                        name: "vaultwarden",
                        relatives: [
                            { kind: "Pod", name: "vaultwarden-8599497567-s7rqj", relatives: [] },
                        ],
                    },
                ],
            },
        ],
    },
    {
        kind: "Namespace",
        name: "excalidraw",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "excalidraw",
                relatives: [
                    {
                        kind: "Service",
                        name: "excalidraw",
                        relatives: [
                            { kind: "Pod", name: "excalidraw-5dd8cd7846-hz8dz", relatives: [] },
                        ],
                    },
                ],
            },
        ],
    },
    {
        kind: "Namespace",
        name: "crowdsec",
        relatives: [
            { kind: "Service", name: "crowdsec-service", relatives: [] },
            { kind: "Service", name: "crowdsec-agent-service", relatives: [] },
            { kind: "Service", name: "crowdsec-appsec-service", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "mealie",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "mealie-int",
                relatives: [
                    {
                        kind: "Service",
                        name: "mealie",
                        relatives: [
                            { kind: "Pod", name: "mealie-556c4fb458-4f599", relatives: [] },
                        ],
                    },
                ],
            },
            {
                kind: "HTTPRoute",
                name: "mealie",
                relatives: [
                    {
                        kind: "Service",
                        name: "mealie",
                        relatives: [
                            { kind: "Pod", name: "mealie-556c4fb458-4f599", relatives: [] },
                        ],
                    },
                ],
            },
        ],
    },
    {
        kind: "Namespace",
        name: "metallb-system",
        relatives: [
            { kind: "Pod", name: "speaker-cj4fc", relatives: [] },
            { kind: "Pod", name: "speaker-25xfp", relatives: [] },
            { kind: "Pod", name: "speaker-ln6tx", relatives: [] },
            { kind: "Pod", name: "speaker-ncsrq", relatives: [] },
            { kind: "Pod", name: "speaker-m64dn", relatives: [] },
            { kind: "Pod", name: "speaker-rhwhr", relatives: [] },
            { kind: "Service", name: "metallb-webhook-service", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "envoy-gateway-system",
        relatives: [
            { kind: "Service", name: "homelab-gateway", relatives: [] },
            { kind: "Service", name: "homelab-gateway-metallb", relatives: [] },
            { kind: "Service", name: "envoy-gateway", relatives: [] },
            { kind: "Service", name: "homelab-envoy-proxy-bouncer", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "cloudflared",
        relatives: [
            { kind: "Pod", name: "cloudflared-65486cd86f-7qfrx", relatives: [] },
        ],
    },
    {
        kind: "Namespace",
        name: "kube-system",
        relatives: [
            {
                kind: "HTTPRoute",
                name: "headlamp",
                relatives: [
                    {
                        kind: "Service",
                        name: "headlamp",
                        relatives: [
                            { kind: "Pod", name: "headlamp-75978c6fc-lwp2q", relatives: [] },
                        ],
                    },
                ],
            },
            { kind: "Pod", name: "local-path-provisioner-5b5f758bcf-wp9nn", relatives: [] },
            { kind: "Pod", name: "vpa-updater-79bff8894f-dhvcm", relatives: [] },
            { kind: "Pod", name: "vpa-admission-controller-54db474db9-25xfz", relatives: [] },
            { kind: "Pod", name: "vpa-recommender-85d6b6c498-vck67", relatives: [] },
            { kind: "Service", name: "kube-dns", relatives: [] },
            { kind: "Service", name: "sealed-secrets-controller-metrics", relatives: [] },
            { kind: "Service", name: "prometheus-kube-prometheus-kube-controller-manager", relatives: [] },
            { kind: "Service", name: "prometheus-kube-prometheus-kube-scheduler", relatives: [] },
            { kind: "Service", name: "prometheus-kube-prometheus-kube-proxy", relatives: [] },
            { kind: "Service", name: "metrics-server", relatives: [] },
            { kind: "Service", name: "monitoring-stack-kube-prom-kubelet", relatives: [] },
            { kind: "Service", name: "prometheus-kube-prometheus-coredns", relatives: [] },
            { kind: "Service", name: "prometheus-kube-prometheus-kube-etcd", relatives: [] },
            { kind: "Service", name: "monitoring-kube-prometheus-kubelet", relatives: [] },
            { kind: "Service", name: "sealed-secrets-controller", relatives: [] },
            { kind: "Service", name: "prometheus-kube-prometheus-kubelet", relatives: [] },
        ],
    },
];
