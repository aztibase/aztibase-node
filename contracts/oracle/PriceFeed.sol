// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Aztibase Price Feed Oracle
/// @notice Stores signed price updates for asset pairs. Any contract can read prices.
/// @dev Phase 1: trusted updater model. Phase 2: RedStone signed data verification.
contract PriceFeed {
    struct PriceData {
        uint256 price;        // Price scaled to 8 decimals (e.g. 350000000000 = $3,500.00)
        uint256 timestamp;    // Unix timestamp of the price update
        uint256 blockNumber;  // Block number when updated
    }

    uint8 public constant DECIMALS = 8;

    address public owner;
    mapping(address => bool) public updaters;
    mapping(bytes32 => PriceData) public prices;
    bytes32[] public feedIds;
    mapping(bytes32 => bool) private feedExists;

    event PriceUpdated(bytes32 indexed feedId, uint256 price, uint256 timestamp);
    event UpdaterAdded(address indexed updater);
    event UpdaterRemoved(address indexed updater);
    event OwnerTransferred(address indexed previousOwner, address indexed newOwner);

    modifier onlyOwner() {
        require(msg.sender == owner, "not owner");
        _;
    }

    modifier onlyUpdater() {
        require(updaters[msg.sender] || msg.sender == owner, "not updater");
        _;
    }

    constructor() {
        owner = msg.sender;
        updaters[msg.sender] = true;
    }

    function addUpdater(address updater) external onlyOwner {
        updaters[updater] = true;
        emit UpdaterAdded(updater);
    }

    function removeUpdater(address updater) external onlyOwner {
        updaters[updater] = false;
        emit UpdaterRemoved(updater);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "zero address");
        emit OwnerTransferred(owner, newOwner);
        owner = newOwner;
    }

    /// @notice Update a single price feed
    /// @param feedId Feed identifier (e.g. keccak256("AZTB/USD"))
    /// @param price Price scaled to 8 decimals
    /// @param timestamp Unix timestamp of the price observation
    function updatePrice(bytes32 feedId, uint256 price, uint256 timestamp) external onlyUpdater {
        require(price > 0, "zero price");
        require(timestamp > 0, "zero timestamp");
        require(timestamp <= block.timestamp + 60, "future timestamp");

        PriceData storage current = prices[feedId];
        require(timestamp >= current.timestamp, "stale update");

        current.price = price;
        current.timestamp = timestamp;
        current.blockNumber = block.number;

        if (!feedExists[feedId]) {
            feedIds.push(feedId);
            feedExists[feedId] = true;
        }

        emit PriceUpdated(feedId, price, timestamp);
    }

    /// @notice Batch update multiple price feeds
    function updatePrices(
        bytes32[] calldata _feedIds,
        uint256[] calldata _prices,
        uint256[] calldata _timestamps
    ) external onlyUpdater {
        require(_feedIds.length == _prices.length && _prices.length == _timestamps.length, "length mismatch");

        for (uint256 i = 0; i < _feedIds.length; i++) {
            require(_prices[i] > 0, "zero price");

            PriceData storage current = prices[_feedIds[i]];
            if (_timestamps[i] < current.timestamp) continue;

            current.price = _prices[i];
            current.timestamp = _timestamps[i];
            current.blockNumber = block.number;

            if (!feedExists[_feedIds[i]]) {
                feedIds.push(_feedIds[i]);
                feedExists[_feedIds[i]] = true;
            }

            emit PriceUpdated(_feedIds[i], _prices[i], _timestamps[i]);
        }
    }

    /// @notice Get the latest price for a feed
    /// @return price The price scaled to 8 decimals
    /// @return timestamp When the price was observed
    /// @return blockNumber Block when the price was stored on-chain
    function getPrice(bytes32 feedId) external view returns (uint256 price, uint256 timestamp, uint256 blockNumber) {
        PriceData storage data = prices[feedId];
        return (data.price, data.timestamp, data.blockNumber);
    }

    /// @notice Get the latest price, reverts if stale
    /// @param feedId Feed identifier
    /// @param maxAge Maximum age in seconds before price is considered stale
    function getFreshPrice(bytes32 feedId, uint256 maxAge) external view returns (uint256 price, uint256 timestamp) {
        PriceData storage data = prices[feedId];
        require(data.price > 0, "no price");
        require(block.timestamp - data.timestamp <= maxAge, "stale price");
        return (data.price, data.timestamp);
    }

    /// @notice Get all registered feed IDs
    function getAllFeeds() external view returns (bytes32[] memory) {
        return feedIds;
    }

    /// @notice Get number of registered feeds
    function feedCount() external view returns (uint256) {
        return feedIds.length;
    }
}
