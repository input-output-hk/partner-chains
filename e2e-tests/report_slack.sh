#!/bin/bash

slack_webhook_url=$SLACK_WEBHOOK_URL
jira_url=$JIRA_URL
report=".report.json"
repository=$1
ref_name=$2
job_url=$3
xray_id=$6

xray_exec_url="${jira_url}/browse/${xray_id}"

# general stats
exit_code="$(cat ${report} | jq '.exitcode')"
total="$(cat ${report} | jq '.summary.total')"
collected="$(cat ${report} | jq '.summary.collected')"
deselected="$(cat ${report} | jq '.summary.deselected')"
duration="$(cat ${report} | jq '.duration')"
duration_rounded_down=$(echo "scale=0; $duration / 1" | bc)

# test results, default to 0
passed="$(cat ${report} | jq '.summary.passed // 0')"
xpassed="$(cat ${report} | jq '.summary.xpassed // 0')"
failed="$(cat ${report} | jq '.summary.failed // 0')"
xfailed="$(cat ${report} | jq '.summary.xfailed // 0')"
errors="$(cat ${report} | jq '.summary.error // 0')"
skipped="$(cat ${report} | jq '.summary.skipped // 0')"

msg=""
add_part() {
    if [[ $1 -gt 0 ]]; then
        if [ -n "$msg" ]; then
            msg+=", "
        fi
        msg+="$1 $2"
    fi
}

add_part $errors "errors"
add_part $failed "failed"
add_part $xfailed "xfailed"
add_part $skipped "skipped"
add_part $passed "passed"
add_part $xpassed "xpassed"

msg="======= $msg in ${duration_rounded_down}s ======="
msg+="\nTest Environment: $env"
msg+="\nTriggered by: $github_actor_username"

if [[ $exit_code -eq 0 ]]
then
    color="#2EB67D"
    job_status="passed"
else
    color="#E01E5A"
    job_status="failed"
fi

fields="{
    \"type\": \"mrkdwn\",
    \"text\": \"<$job_url|CI job>\"
}"

if [[ "$jira_url" == *"https://"* ]] && [ -n "$xray_id" ] && [ "$xray_id" != "null" ]; then
    fields+=",{
        \"type\": \"mrkdwn\",
        \"text\": \"<$xray_exec_url|Xray report>\"
    }"
fi

echo Posting slack notification: "$msg"
json_data="{
    \"username\": \"partner-chains-tests bot\",
    \"icon_emoji\": \":robot_face:\",
    \"attachments\": [
        {
            \"blocks\": [
                {
                    \"type\": \"header\",
                    \"text\": {
                        \"type\": \"plain_text\",
                        \"text\": \"Tests $job_status! ($repository: $ref_name)\"
                    }
                },
                {
                    \"type\": \"section\",
                    \"text\": {
                        \"type\": \"mrkdwn\",
                        \"text\": \"$msg\"
                    },
                    \"fields\": [
                        $fields
                    ]
                }
            ],
            \"color\": \"$color\"
        }
    ]
}"

curl --request POST \
  --url $slack_webhook_url \
  --header 'Content-Type: application/json' \
  --data "$json_data" --silent
