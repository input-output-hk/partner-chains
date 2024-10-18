#!/bin/bash

XRAY_URL=$XRAY_API_BASE_URL
CLIENT_ID=$XRAY_CLIENT_ID
CLIENT_SECRET=$XRAY_CLIENT_SECRET

while getopts "p:e:r:" opt; do
  case $opt in
    p) TEST_PLAN_KEY=$OPTARG ;;
    e) TEST_EXEC_KEY=$OPTARG ;;
    r) REPORT_PATH=$OPTARG ;;
    \?) echo "Usage: cmd [-p] [-e] [-r]" ;;
  esac
done

echo "------------------SEND XRAY TEST RESULTS------------------"

AUTH_DATA="{\"client_id\": \"${CLIENT_ID}\", \"client_secret\": \"${CLIENT_SECRET}\"}"

AUTH_TOKEN=$(curl -X POST "${XRAY_URL}/authenticate" \
  -H "Content-type: application/json" \
  -H "Accept: text/plain" \
  --data "$AUTH_DATA" --silent)

echo "Receiving auth token for XRay"

if [ "$TEST_PLAN_KEY" ]; then
    SEND_PARAM="testPlanKey=${TEST_PLAN_KEY}"
else
    SEND_PARAM="testExecKey=${TEST_EXEC_KEY}"
fi

echo "Uploading XRay test results to ${SEND_PARAM}"

SEND_RESPONSE=$(curl -X POST "${XRAY_URL}/import/execution/junit?${SEND_PARAM}&projectKey=ETCM" \
 -H "Content-type: application/xml" \
 -H "Authorization: Bearer ${AUTH_TOKEN//\"}" \
 --data @"$REPORT_PATH" \
  --silent)

echo "XRAY API responded with: ${SEND_RESPONSE}"
EXECUTION_ID=$(echo $SEND_RESPONSE | grep -o '"key":"[^"]*"' | sed 's/"key":"\([^"]*\)"/\1/')
echo $EXECUTION_ID > xray_id.txt