# Tapfer

# Installation and setup

1. Clone repo
2. edit `docker-compose.yml` HOST variable to match the domain that uploads will be available on (for QR code generation)
3. Configure your reverse proxy if applicable according to `reverse_proxy.md`
4. (optional) Configure a ZFS storage quota (or similar) on the `data` folder or keep the data a volume without a local mountpoint
5. `docker-compose up -d --build` To build and deploy the container