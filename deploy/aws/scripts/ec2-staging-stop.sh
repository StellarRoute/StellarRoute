#!/usr/bin/env bash
# Stop the staging EC2 instance and wait until it is stopped.
#
# Usage:
#   bash deploy/aws/scripts/ec2-staging-stop.sh
#
# Optional overrides:
#   AWS_REGION=us-east-1 STAGING_INSTANCE_ID=i-xxxxxxxx bash deploy/aws/scripts/ec2-staging-stop.sh
#   AWS_REGION=us-east-1 STAGING_INSTANCE_NAME=stellarroute-staging-ec2 bash deploy/aws/scripts/ec2-staging-stop.sh
set -euo pipefail

AWS_REGION="${AWS_REGION:-us-east-1}"
STAGING_INSTANCE_ID="${STAGING_INSTANCE_ID:-}"
STAGING_INSTANCE_NAME="${STAGING_INSTANCE_NAME:-stellarroute-staging-ec2}"

resolve_instance_id() {
  if [[ -n "${STAGING_INSTANCE_ID}" ]]; then
    echo "${STAGING_INSTANCE_ID}"
    return 0
  fi

  aws ec2 describe-instances \
    --region "${AWS_REGION}" \
    --filters \
      "Name=tag:Name,Values=${STAGING_INSTANCE_NAME}" \
      "Name=instance-state-name,Values=running,stopped,stopping,pending" \
    --query 'Reservations[].Instances[].InstanceId' \
    --output text
}

INSTANCE_ID="$(resolve_instance_id)"

if [[ -z "${INSTANCE_ID}" || "${INSTANCE_ID}" == "None" ]]; then
  echo "Could not find staging instance. Set STAGING_INSTANCE_ID or verify Name tag ${STAGING_INSTANCE_NAME}." >&2
  exit 1
fi

CURRENT_STATE="$(aws ec2 describe-instances \
  --region "${AWS_REGION}" \
  --instance-ids "${INSTANCE_ID}" \
  --query 'Reservations[0].Instances[0].State.Name' \
  --output text)"

if [[ "${CURRENT_STATE}" == "stopped" ]]; then
  echo "Instance ${INSTANCE_ID} is already stopped."
  exit 0
fi

echo "Stopping instance ${INSTANCE_ID} in ${AWS_REGION}..."
aws ec2 stop-instances --region "${AWS_REGION}" --instance-ids "${INSTANCE_ID}" >/dev/null
aws ec2 wait instance-stopped --region "${AWS_REGION}" --instance-ids "${INSTANCE_ID}"

echo "Staging instance stopped."
echo "  Instance ID: ${INSTANCE_ID}"
