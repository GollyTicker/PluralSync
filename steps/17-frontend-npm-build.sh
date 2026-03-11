#/bin/bash

set -euo pipefail

./steps/14-frontend-generate-announcements.sh
./steps/15-frontend-generate-bindings.sh

cd frontend && npm run build

