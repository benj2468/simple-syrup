// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

contract Counter {
    // All Code goes here
    string public name = "Counter";

    uint256 public counter = 0;

    constructor() {}

    // Increase the counter
    function inc() public {
        counter += 1;
    }

    // Decrease the counter
    function dec() public {
        counter -= 1;
    }
}
