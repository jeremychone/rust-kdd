
**IMPORTANT** Still under developmment.

## Intro and Concepts

**kdd** is a command-line utility to streamline Kubernetes-driven development and deployment. It does NOT have any runtime components or even builds ones. **kdd** is just a way to structure a cloud application into a multi-service Kubernetes oriented system and streamline docker, kubectl, and cloud cli into a single command line set. 

Each system have one `kdd.yaml` file, with two main constructs:

- **blocks** are things that get build
- **builds** are one or more command instructions to build blocks given files in their directory
- **realms** are places (i.e., k8s cluster) to where parts get deployed

The `kdd.yaml` is at the root of the sytem with the following model

```yaml
system: my-big-app
block_root: services/
blocks:
  - web_server
  - agent
  - redis_queue
  - db

realms:
  - local_dev

```

Example of future commands

```sh
# Will generate output from the handlebars k8s/... files
kdd ktemplate

# Change realms
kdd realm local

# Build docker blocks (and their dependencies)
kdd dbuild

# docker push docker images to the current realm
kdd dpush

```