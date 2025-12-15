#!/bin/sh
echo "$(date) - Args: $@" >> /tmp/mcp_debug.log
echo "Environment: " >> /tmp/mcp_debug.log
env >> /tmp/mcp_debug.log
exec /Users/shyalter/Code/puemos/lareview/target/debug/lareview "$@"
