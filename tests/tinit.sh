#!/bin/sh
# /bin/init-test.sh - Init System Integration Test Suite

FAILED=0

log_pass() {
    echo "  [PASS] $1"
}

log_fail() {
    echo "  [FAIL] $1"
    FAILED=1
}

echo "========================================="
echo "   Running Init System Integration Tests"
echo "========================================="

# Test 1: Verify Essential VFS Mounts
echo "--- Testing Virtual Filesystem Mounts ---"
for mp in /proc /sys /dev /dev/pts /run; do
    if mountpoint -q "$mp"; then
        log_pass "$mp is mounted"
    else
        log_fail "$mp is missing or not mounted"
    fi
done

# Test 2: Verify Network Loopback
echo "--- Testing Network Stack ---"
if ip link show lo 2>/dev/null | grep -q "UP"; then
    log_pass "Loopback interface (lo) is UP"
else
    log_fail "Loopback interface (lo) is down or missing"
fi

# Test 3: Verify Root Filesystem Writable State
echo "--- Testing Root Filesystem ---"
if touch /test_write_check 2>/dev/null && rm /test_write_check 2>/dev/null; then
    log_pass "Root filesystem is read-write"
else
    log_fail "Root filesystem is read-only"
fi

# Test 4: Verify PID 1 Status
echo "--- Testing Process Context ---"
MY_PID=$(awk '{print $1}' /proc/self/stat)
if [ "$MY_PID" -eq 1 ]; then
    log_pass "Test harness is running as PID 1 (or direct child)"
else
    echo "  [INFO] Running as PID $MY_PID (Child of init)"
fi

echo "========================================="
if [ $FAILED -eq 0 ]; then
    echo " RESULT: ALL TESTS PASSED SUCCESSFULLY!"
    echo "========================================="
    # Optional: Automatically shut down after successful test run
    # exec poweroff -f
    exit 0
else
    echo " RESULT: SOME TESTS FAILED!"
    echo "========================================="
    exit 1
fi