#!/bin/sh

### returns respective DOCKER TAG
### release-tag => <release-tag> (e.g. v1.2.3)
### schedule => <SCHEDULE_CADENCE> (e.g. nightly)
### master => "master"
### else => ""
publishdockertag() {
  # This stores the result in PUBLISH_DOCKER_TAG.
  export PUBLISH_DOCKER_TAG="";

  if [ -n "${CI_COMMIT_TAG}" ] && echo "${CI_COMMIT_TAG}" | grep -Eq "${TAG_REGEX}"; then
    export PUBLISH_DOCKER_TAG="${CI_COMMIT_TAG}";
    return 0
  fi

  if [ -n "${SCHEDULE_CADENCE}" ] && [ "${CI_PIPELINE_SOURCE}" = "schedule" ]; then
    # Sanitize check.
    if echo "${SCHEDULE_CADENCE}" | grep -Eq "${TAG_REGEX}"; then
      export PUBLISH_DOCKER_TAG="invalid_cadence";
    else
      export PUBLISH_DOCKER_TAG="${SCHEDULE_CADENCE}";
    fi
    return 0;
  fi

  if [ -n "${CI_DEFAULT_BRANCH}" ] && [ "${CI_COMMIT_BRANCH}" = "${CI_DEFAULT_BRANCH}" ]; then
    export PUBLISH_DOCKER_TAG="master";
    return 0;
  fi
}
