// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./AztibasePair.sol";

/// @title Aztibase AMM Factory
/// @notice Creates and tracks all trading pair contracts.
contract AztibaseFactory {
    address public feeTo;
    address public owner;

    mapping(address => mapping(address => address)) public getPair;
    address[] public allPairs;

    event PairCreated(address indexed token0, address indexed token1, address pair, uint256 index);
    event FeeToUpdated(address indexed feeTo);
    event OwnerUpdated(address indexed owner);

    constructor() {
        owner = msg.sender;
    }

    function allPairsLength() external view returns (uint256) {
        return allPairs.length;
    }

    function createPair(address tokenA, address tokenB) external returns (address pair) {
        require(tokenA != tokenB, "identical addresses");
        (address token0, address token1) = tokenA < tokenB ? (tokenA, tokenB) : (tokenB, tokenA);
        require(token0 != address(0), "zero address");
        require(getPair[token0][token1] == address(0), "pair exists");

        AztibasePair newPair = new AztibasePair();
        newPair.initialize(token0, token1);
        pair = address(newPair);

        getPair[token0][token1] = pair;
        getPair[token1][token0] = pair;
        allPairs.push(pair);

        emit PairCreated(token0, token1, pair, allPairs.length);
    }

    function setFeeTo(address _feeTo) external {
        require(msg.sender == owner, "forbidden");
        feeTo = _feeTo;
        emit FeeToUpdated(_feeTo);
    }

    function setOwner(address _owner) external {
        require(msg.sender == owner, "forbidden");
        owner = _owner;
        emit OwnerUpdated(_owner);
    }
}
