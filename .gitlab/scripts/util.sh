#!/bin/bash

### returns respective DOCKER TAG
### release-tag => <release-tag> (e.g. v1.2.3)
### schedule => <SCHEDULE_CADENCE> (e.g. nightly)
### master => "master"
### else => ""
publishdockertag () {
# this stores the result in PUBLISH_DOCKER_TAG
export PUBLISH_DOCKER_TAG="";
if [[ "${CI_COMMIT_TAG}" =~ ${TAG_REGEX} ]]; then
  export PUBLISH_DOCKER_TAG="${CI_COMMIT_TAG}";
  return 0;
fi
if [[ "${SCHEDULE_CADENCE}" != "" && ${CI_PIPELINE_SOURCE} == "schedule" ]]; then
  # sanitize check
  if [[ "${SCHEDULE_CADENCE}" =~ ${TAG_REGEX} ]]; then
    export PUBLISH_DOCKER_TAG="invalid_cadence";
  else
    export PUBLISH_DOCKER_TAG="${SCHEDULE_CADENCE}";
  fi
  return 0;
fi
if [[ ${CI_COMMIT_BRANCH} == ${CI_DEFAULT_BRANCH} ]]; then
  export PUBLISH_DOCKER_TAG="master";
  return 0;
fi
}