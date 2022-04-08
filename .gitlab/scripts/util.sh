#!/bin/bash

### returns respective DOCKER TAG
### release-tag => <release-tag> (e.g. v1.2.3)
### schedule => <SCHEDULE_CADENCE> (e.g. nightly)
### master => "master"
### else => ""
publishdockertag () {
# this stores the result in a variable defined by the caller
local -n VAR=$1
VAR="";
if [[ "${CI_COMMIT_TAG}" =~ ${TAG_REGEX} ]]; then
  VAR="${CI_COMMIT_TAG}";
  return 0;
fi
if [[ -z "${SCHEDULE_CADENCE}" && ${CI_PIPELINE_SOURCE} == "schedule" ]]; then
  # sanitize check
  if [[ "${SCHEDULE_CADENCE}" =~ ${TAG_REGEX} ]]; then
    VAR="invalid_cadence";
  else
    VAR="${SCHEDULE_CADENCE}";
  fi
  return 0;
fi
if [[ ${CI_COMMIT_BRANCH} == ${CI_DEFAULT_BRANCH} ]]; then
  VAR="master";
  return 0;
fi
}