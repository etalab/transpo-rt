#!/usr/bin/env bash
set -e
echo Preparing configuration (fetched from GitHub)

#
# NOTE: a deploy key must be generated following these instructions. The public part
# must be added to etalab/transport_deploy as an authorised deploy key with read-only access.
#
# https://docs.github.com/en/developers/overview/managing-deploy-keys#deploy-keys
#

# TODO: use base64 encoding to store the private key in ENV, then use it to achieve the clone 
# https://community.netlify.com/t/support-guide-using-an-ssh-key-via-environment-variable-during-build/2457

CONFIG_REPO=git@github.com:etalab/deploy.git
CONFIG_FOLDER=transport_deploy_config

echo "Cloning repository $CONFIG_REPO"
git clone --depth 1 --no-checkout --filter=blob:none $CONFIG_REPO $CONFIG_FOLDER
cd $CONFIG_FOLDER
git checkout master -- transpo_rt/prod.yml

# TODO: modify the CC configuration to:
# * Use a pre-run hook to run fetch_configuration.sh
# * Modify ENV to point TRANSPO_RT_CONFIG_FILE to $CONFIG_FOLDER/transpo_rt/prod.yml
