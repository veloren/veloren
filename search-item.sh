#!/bin/bash

for item in $(cat list.txt);do
	if [ -z "$(grep -RIin --exclude-dir=\.git --exclude=list.txt $item)" ]; then
		echo $item
	fi
done
