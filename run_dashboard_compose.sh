#!/bin/bash

# Navigate to the root directory of the project
cd "$(dirname "$0")"

# Build the worker-server application
echo "Building worker-server application..."
cd worker-server
if ! cargo build; then
    echo "Failed to build worker-server application. Exiting..."
    exit 1
fi
cd ..

# Build the load-balancer application
echo "Building load-balancer application..."
cd load-balancer
if ! cargo build; then
    echo "Failed to build load-balancer application. Exiting..."
    exit 1
fi
cd ..

# Build and run the dashboard application
echo "Building and running dashboard application..."
cd dashboard
APP_ENVIRONMENT=docker-compose
if ! cargo run; then
    echo "Failed to build and run dashboard application. Exiting..."
    exit 1
fi

echo "All applications built successfully."
