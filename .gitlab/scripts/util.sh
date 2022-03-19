#!/bin/bash

### forbid setting SCHEDULE_CADENCE to v0.1.2 syntax
sanitize () {
local -n VAR=$1
INPUT=$2
if [[ "${INPUT}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
  echo "---WARN---SANITIZE---ACTIVE---"
  VAR="";
  return 1;
fi
VAR="${INPUT}";
}

### returns the respective type of publish for the ci
### 0 => optional build/no publish
### 10 => master build
### 20 => scheduled build
### 30 => release build
publishtype () {
local -n VAR=$1
if [[ "${CI_COMMIT_TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
  VAR=30;
  return 0;
fi
if [[ -z "${SCHEDULE_CADENCE}" && ${CI_PIPELINE_SOURCE} == "schedule" ]]; then
  VAR=20;
  return 0;
fi
if [[ ${CI_COMMIT_BRANCH} == ${CI_DEFAULT_BRANCH} ]]; then
  VAR=10;
  return 0;
fi
VAR=0;
}

### returns respective DOCKER TAG
### 0 => exit code 3
### 10 => master
### 20 => <SCHEDULE_CADENCE>
### 30 => <tag>
### else => exit code 7

publishdockertag () {
local -n VAR=$1
INPUT=$2
if [[ "${INPUT}" == "0" ]]; then
  VAR="";
  return 3;
fi
if [[ "${INPUT}" == "10" ]]; then
  VAR="master";
  return 0;
fi
if [[ "${INPUT}" == "20" ]]; then
  VAR="${SCHEDULE_CADENCE}";
  sanitize $1 "${VAR}";
  return 0;
fi
if [[ "${INPUT}" == "30" ]]; then
  VAR="${CI_COMMIT_TAG}";
  return 0;
fi
VAR="";
return 7;
}

### returns respective GIT TAG
### 0 => exit code 3
### 10 => exit code 4
### 20 => <SCHEDULE_CADENCE>
### 30 => exit code 6
### else => exit code 7

publishgittag () {
local -n VAR=$1
INPUT=$2
if [[ "${INPUT}" == "0" ]]; then
  VAR="";
  return 3;
fi
if [[ "${INPUT}" == "10" ]]; then
  VAR="";
  return 4;
fi
if [[ "${INPUT}" == "20" ]]; then
  VAR="${SCHEDULE_CADENCE}";
  sanitize $1 "${VAR}";
  return 0;
fi
if [[ "${INPUT}" == "30" ]]; then
  VAR="${CI_COMMIT_TAG}";
  return 0;
fi
VAR="";
return 7;
}