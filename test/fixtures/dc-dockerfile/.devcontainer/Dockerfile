FROM public.ecr.aws/docker/library/ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*
RUN echo "pelagos-dockerfile-build" > /pelagos-marker.txt
