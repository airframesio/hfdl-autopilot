#!/bin/bash

SCRIPT_ROOT="$(dirname -- "${BASH_SOURCE[0]}")"
PYTHON_ENV=${SCRIPT_ROOT}/venv
SCRIPT_PATH=${SCRIPT_ROOT}/convert_table.py

if [ ! -d "${PYTHON_ENV}" ]; then
  python3 -m venv ${PYTHON_ENV}
  source ${PYTHON_ENV}/bin/activate
  pip3 install --upgrade pip >/dev/null
  pip3 install -r requirements.txt >/dev/null
else
  source ${PYTHON_ENV}/bin/activate
fi

python3 ${SCRIPT_PATH} $*
