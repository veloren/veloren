#! /bin/bash

- git fetch origin master
- git fetch origin master

- BRANCH_NAME=$(git branch --contains HEAD | tail -1 | cut -c 3-)
- echo "Working on     ::" $BRANCH_NAME
- LATEST_COMMIT=$(git show-ref --heads -s master)
- echo "Master commit  ::" $LATEST_COMMIT
- SIMILAR_COMMIT=$(git merge-base master $BRANCH_NAME)
- echo "Similar commit ::" $SIMILAR_COMMIT
- if [ "$LATEST_COMMIT" != "$SIMILAR_COMMIT" ]; then
        echo "These commits should be the same, so this branch needs to be rebased!";
        echo "If you need help, or something is wrong, message @AngelOnFira!";
        exit 1;
    else
        echo "This branch appears to be rebased :100:"
    fi