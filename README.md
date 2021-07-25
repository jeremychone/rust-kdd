
## Intro and Concepts

**kdd** is a command-line utility to streamline Kubernetes-driven development and deployment. It does NOT have any runtime components or even builds ones. **kdd** is just a way to structure a cloud application into a multi-service Kubernetes oriented system and streamline docker, kubectl, and cloud cli into a single command line set. 

Each system have one `kdd.yaml` file, with two main constructs:

- **blocks** are things that get build
- **realms** are places (i.e., k8s cluster) to where parts get deployed
- **builds** are one or more command instructions to build blocks given files in their directory

## Install

**From cargo (recommended for now):**
```sh 
cargo install kdd
```

Advanced: [Install from Binary](#install-with-binst)

## Example

The `kdd.yaml` is at the root of the sytem with the following model.

Full example at [Cloud Starter](https://github.com/BriteSnow/cloud-starter)

```yaml
system: cstar
image_tag: "{{__version__}}"
block_base_dir: services/ 

blocks:
  - _common
  - web_server
  - agent
  - redis_queue
  - db
  - name: web 
    dir: frontends/web/
  - name: web-server
    dependencies: ['_common','web'] # build dependency for when running dbuild (no effect on build).  

realms:
  _base_:  
    web_external_ports: 8080
    
  dev:
    yaml_dir: k8s/dev/ # for dev, we override the yamlDir
    context: docker-desktop
    dev_stuff: Some dev stuff
    confirm_delete: false

  aws:
    yaml_dir: k8s/aws/
    context: arn:aws:eks:us-west-2:843615417314:cluster/cstar-cluster
    profile: jc-root
    registry: 843615417314.dkr.ecr.us-west-2.amazonaws.com/
    default_configurations: ['agent', 'db', 'queue', 'web-server']
    confirm_delete: false

builders: 
  - name: npm_install
    when_file: ./package.json
    exec:
      cmd: npm
      cmd_type: global # base_dir | block_dir
      args: ["install", "--color"]
  - name: tsc
    when_file: ./tsconfig.json
    exec:
      cmd: node_modules/.bin/tsc
  - name: rollup
    when_file: ./rollup.config.js
    replace: tsc # rollup has rollup-ts, so no need to do it twice
    exec:
      cmd: node_modules/.bin/rollup
      args: ["-c"]
  - name: pcss
    when_file: ./pcss.config.js
    exec:
      cmd: node_modules/.bin/pcss
```

Command examples:

```sh
# Change realms to dev
kdd realm dev

# Build docker blocks (and their dependencies)
kdd dbuild
# build per block name (no space)
kdd dbuild agent,web-server

# docker push docker images to the current realm to the registry
kdd dpush
# push only some docker images
kdd dpush agent,web-server

# execute a kubectl apply for all default configurations
kdd kapply 
# selectively doing kubectl apply (push image before)
kdd kapply web-server,agent

# kdd kdelete, kdd kcreate, kdd kexec ... for the kubectl equivalents
```

## Install with binst

(mac / linux only for now)

[Install binst](https://crates.io/crates/binst) first.

```sh
binst install -r https://binst.io/jc-repo kdd 
```
