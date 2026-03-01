set shell := ["bash", "-euo", "pipefail", "-c"]

app_dir := `pwd`
service_name := "citordle-backend"
service_file := "/etc/systemd/system/citordle-backend.service"
env_file := "/etc/citordle/citordle.env"
run_user := `id -un`

default:
    @just --list

backend:
    cargo run --manifest-path backend/Cargo.toml

frontend:
    bun --cwd frontend dev

build-backend:
    cargo build --release --manifest-path backend/Cargo.toml

build-frontend:
    bun --cwd frontend install --frozen-lockfile
    bun --cwd frontend run build

deploy-frontend: build-frontend
    sudo install -d -m 755 "/var/www/citordle"
    sudo rsync -a --delete "frontend/dist/" "/var/www/citordle/"

build-prod: build-backend build-frontend

init-env:
    sudo install -d -m 750 "/etc/citordle"
    sudo bash -c 'if [ ! -f "{{env_file}}" ]; then umask 027; printf "%s\n" "PORT=8080" "FRONTEND_ORIGIN=https://your-domain.example" "JWT_SECRET=$(openssl rand -hex 32)" > "{{env_file}}"; fi'

rotate-jwt:
    sudo install -d -m 750 "/etc/citordle"
    sudo touch "{{env_file}}"
    sudo chmod 640 "{{env_file}}"
    sudo bash -c 'tmp=$(mktemp); grep -v "^JWT_SECRET=" "{{env_file}}" > "$tmp" || true; printf "JWT_SECRET=%s\n" "$(openssl rand -hex 32)" >> "$tmp"; cat "$tmp" > "{{env_file}}"; rm -f "$tmp"'

install-service:
    sudo bash -c 'printf "%s\n" "[Unit]" "Description=Citordle backend service" "After=network.target" "" "[Service]" "Type=simple" "User={{run_user}}" "WorkingDirectory={{app_dir}}" "EnvironmentFile={{env_file}}" "ExecStart={{app_dir}}/backend/target/release/citordle-backend" "Restart=always" "RestartSec=5" "NoNewPrivileges=true" "PrivateTmp=true" "" "[Install]" "WantedBy=multi-user.target" > "{{service_file}}"; systemctl daemon-reload'

enable:
    sudo systemctl enable "{{service_name}}"

disable:
    sudo systemctl disable "{{service_name}}"

start: rotate-jwt
    sudo systemctl start "{{service_name}}"

restart: rotate-jwt
    sudo systemctl restart "{{service_name}}"

stop:
    sudo systemctl stop "{{service_name}}"

status:
    sudo systemctl status "{{service_name}}" --no-pager

logs:
    sudo journalctl -u "{{service_name}}" -n 200 --no-pager

deploy-backend: build-backend init-env install-service enable restart

deploy: deploy-backend deploy-frontend
