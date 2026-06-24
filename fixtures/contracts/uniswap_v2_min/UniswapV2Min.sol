// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// Minimal Uniswap V2-style AMM used by the `uniswap-v2-swap` custom workload.
//
// It reproduces Uniswap V2's economically and computationally relevant parts:
//   * a constant-product pair with the 0.3% fee invariant and reserve updates,
//   * a router that pulls input via transferFrom and routes through the pair,
//     giving the realistic tx -> router -> token.transferFrom / pair.swap ->
//     token.transfer call graph (4 contracts deep).
//
// It intentionally omits LP tokens, TWAP oracles, flash swaps and the factory
// CREATE2 pair-address derivation, which add deployment complexity without
// changing the per-swap execution cost profile that this workload measures.
//
// Tokens are the existing `TestToken` ERC20 fixture (see erc20 workload).

interface IERC20 {
    function transfer(address to, uint256 value) external returns (bool);
    function transferFrom(address from, address to, uint256 value) external returns (bool);
    function balanceOf(address owner) external view returns (uint256);
}

contract UniswapV2MinPair {
    address public token0;
    address public token1;
    uint112 private reserve0;
    uint112 private reserve1;

    constructor(address _token0, address _token1) {
        token0 = _token0;
        token1 = _token1;
    }

    function getReserves() external view returns (uint112, uint112) {
        return (reserve0, reserve1);
    }

    // Sync reserves to the current balances. Called after liquidity is
    // transferred in to register it (the workload's "add liquidity").
    function mint() external {
        reserve0 = uint112(IERC20(token0).balanceOf(address(this)));
        reserve1 = uint112(IERC20(token1).balanceOf(address(this)));
    }

    // Uniswap V2 swap: the input amount must already be transferred in; the
    // requested output is sent out and the constant-product invariant with the
    // 0.3% fee is enforced.
    function swap(uint256 amount0Out, uint256 amount1Out, address to) external {
        require(amount0Out > 0 || amount1Out > 0, "INSUFFICIENT_OUTPUT");
        (uint112 r0, uint112 r1) = (reserve0, reserve1);
        require(amount0Out < r0 && amount1Out < r1, "INSUFFICIENT_LIQUIDITY");

        if (amount0Out > 0) IERC20(token0).transfer(to, amount0Out);
        if (amount1Out > 0) IERC20(token1).transfer(to, amount1Out);

        uint256 bal0 = IERC20(token0).balanceOf(address(this));
        uint256 bal1 = IERC20(token1).balanceOf(address(this));
        uint256 amount0In = bal0 > r0 - amount0Out ? bal0 - (r0 - amount0Out) : 0;
        uint256 amount1In = bal1 > r1 - amount1Out ? bal1 - (r1 - amount1Out) : 0;
        require(amount0In > 0 || amount1In > 0, "INSUFFICIENT_INPUT");

        uint256 bal0Adjusted = bal0 * 1000 - amount0In * 3;
        uint256 bal1Adjusted = bal1 * 1000 - amount1In * 3;
        require(
            bal0Adjusted * bal1Adjusted >= uint256(r0) * uint256(r1) * (1000 ** 2),
            "K"
        );

        reserve0 = uint112(bal0);
        reserve1 = uint112(bal1);
    }
}

interface IPair {
    function getReserves() external view returns (uint112, uint112);
    function token0() external view returns (address);
    function swap(uint256 amount0Out, uint256 amount1Out, address to) external;
}

contract UniswapV2MinRouter {
    function getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut)
        public
        pure
        returns (uint256)
    {
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        return numerator / denominator;
    }

    function swapExactTokensForTokens(address pair, address tokenIn, uint256 amountIn, address to)
        external
    {
        IERC20(tokenIn).transferFrom(msg.sender, pair, amountIn);
        (uint112 r0, uint112 r1) = IPair(pair).getReserves();
        if (tokenIn == IPair(pair).token0()) {
            uint256 out = getAmountOut(amountIn, r0, r1);
            IPair(pair).swap(0, out, to);
        } else {
            uint256 out = getAmountOut(amountIn, r1, r0);
            IPair(pair).swap(out, 0, to);
        }
    }
}
