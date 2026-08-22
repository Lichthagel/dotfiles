#!/usr/bin/env bash

test_failures=0

fail_test() {
    printf 'FAIL: %s\n' "$1" >&2
    test_failures=$((test_failures + 1))
}

assert_contains() {
    local value="$1" expected="$2"
    printf '%s' "$value" | grep -Fq -- "$expected" || fail_test "expected output to contain: $expected"
}

assert_not_contains() {
    local value="$1" unexpected="$2"
    printf '%s' "$value" | grep -Fq -- "$unexpected" && fail_test "expected output not to contain: $unexpected"
}

assert_file_exists() {
    [ -e "$1" ] || [ -L "$1" ] || fail_test "expected file to exist: $1"
}

assert_file_not_exists() {
    [ ! -e "$1" ] && [ ! -L "$1" ] || fail_test "expected file not to exist: $1"
}

assert_equals() {
    local actual="$1" expected="$2"
    [ "$actual" = "$expected" ] || fail_test "expected '$expected', got '$actual'"
}
