@echo off
cd /d %~dp0

echo Building worker-server application...
cd worker-server
cargo build
if %errorlevel% neq 0 (
    echo Failed to build worker-server application. Exiting...
    exit /b %errorlevel%
)
cd ..

echo Building load-balancer application...
cd load-balancer
cargo build
if %errorlevel% neq 0 (
    echo Failed to build load-balancer application. Exiting...
    exit /b %errorlevel%
)
cd ..

echo Building and running dashboard application...
cd dashboard
SET APP_ENVIRONMENT=docker-compose
cargo run
if %errorlevel% neq 0 (
    echo Failed to build and run dashboard application. Exiting...
    exit /b %errorlevel%
)

echo All applications built successfully.