# 🛡️ DeFi Risk Monitor - Frontend Dashboard

**Advanced DeFi Risk Monitoring & Portfolio Management Platform**

A comprehensive Next.js-based dashboard for real-time DeFi position monitoring, risk analytics, MEV protection, and cross-chain portfolio management. Built with modern Web3 technologies and integrated with a powerful Rust backend.

## 🚀 Features

### 📊 **Real-Time Risk Analytics**
- **Position Risk Scoring**: Advanced algorithms analyze liquidity, volatility, protocol, MEV, and cross-chain risks
- **Impermanent Loss Tracking**: Real-time IL calculations with historical trends
- **MEV Protection**: Sandwich attack detection and front-running risk assessment
- **Cross-Chain Risk Analysis**: Multi-chain position monitoring and bridge risk evaluation

### 💼 **Portfolio Management**
- **Multi-Chain Support**: Ethereum, Polygon, Arbitrum, Optimism, BSC, Avalanche
- **Position Tracking**: Comprehensive position management with performance analytics
- **P&L Analysis**: Detailed profit/loss tracking with fee breakdowns
- **Asset Allocation**: Diversification analysis and concentration risk monitoring

### 🔔 **Smart Alerts & Monitoring**
- **Customizable Thresholds**: Set alerts for risk levels, price movements, and portfolio changes
- **Real-Time Notifications**: WebSocket-powered instant updates
- **Protocol Event Tracking**: Monitor exploits, governance changes, and market events
- **Emergency Alerts**: Critical risk notifications with recommended actions

### 📈 **Advanced Analytics**
- **Risk Correlation Matrix**: Understand how your positions correlate
- **Volatility Analysis**: Historical and predicted volatility metrics
- **Liquidity Depth Analysis**: Pool liquidity and slippage assessments
- **Performance Benchmarking**: Compare against market indices and strategies

## 🛠️ Tech Stack

- **Frontend**: Next.js 13, React 18, TypeScript
- **Styling**: Tailwind CSS, Ant Design
- **Web3**: Wagmi, Viem, RainbowKit
- **State Management**: React Query (TanStack Query)
- **Charts**: Ant Design Charts
- **Real-time**: WebSockets
- **Backend Integration**: Rust-powered API

## 🚀 Quick Start

### Prerequisites

- **Node.js**: v18.17.1 or later
- **NPM**: 8.19.2 or later
- **Git**: Latest version
- **Web3 Wallet**: MetaMask, WalletConnect, or similar

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/defi-risk-monitor/frontend.git
   cd defi-risk-monitor
   ```

2. **Install dependencies**
   ```bash
   npm install
   # or
   yarn install
   ```

3. **Set up environment variables**
   ```bash
   cp .env.example .env.local
   ```

4. **Configure your environment**
   Edit `.env.local` with your API keys and configuration:
   ```env
   # Backend API
   NEXT_PUBLIC_API_URL=http://localhost:8080/api/v1
   NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws
   
   # Blockchain RPC URLs
   NEXT_PUBLIC_ETHEREUM_RPC_URL=https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY
   NEXT_PUBLIC_POLYGON_RPC_URL=https://polygon-mainnet.alchemyapi.io/v2/YOUR_KEY
   NEXT_PUBLIC_ARBITRUM_RPC_URL=https://arb-mainnet.alchemyapi.io/v2/YOUR_KEY
   
   # WalletConnect Project ID
   NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID=your_project_id
   
   # Optional: Analytics
   NEXT_PUBLIC_ANALYTICS_ID=your_analytics_id
   ```

5. **Start the development server**
   ```bash
   npm run dev
   ```

6. **Open your browser**
   Navigate to [http://localhost:3000](http://localhost:3000)

### Backend Setup

The frontend requires the Rust backend to be running. Please refer to the [backend documentation](../backend/README.md) for setup instructions.

## 📱 Usage

### Connect Your Wallet
1. Click "Connect Wallet" in the top-right corner
2. Choose your preferred wallet (MetaMask, WalletConnect, etc.)
3. Approve the connection request

### Monitor Your Positions
1. Your positions will automatically load after wallet connection
2. View real-time risk scores and analytics
3. Set up custom alert thresholds
4. Monitor cross-chain exposures

### Risk Analysis
1. Click on any position to view detailed risk breakdown
2. Understand impermanent loss projections
3. Review MEV risk assessments
4. Get actionable recommendations

#### ARNK

```
  OPEN: ARNK.COM
  URL: https://www.ARNK.com/
```

## Important Links

- [Get Pro Blockchain Developer Course](https://www.theblockchaincoders.com/pro-nft-marketplace)
- [Support Creator](https://bit.ly/Support-Creator)
- [All Projects Source Code](https://www.theblockchaincoders.com/SourceCode)

## Authors

- [@theblockchaincoders.com](https://www.theblockchaincoders.com/)
- [@consultancy](https://www.theblockchaincoders.com/consultancy)
- [@youtube](https://www.youtube.com/@daulathussain)

# Uniswap Liquidity Creator - Polygon Network

A comprehensive decentralized application (dApp) for creating liquidity pools on Uniswap V3 on the Polygon network. This application allows users to easily create liquidity for newly created tokens and popular token pairs with a user-friendly interface.

## 🌟 Features

- **Custom Token Support**: Add liquidity for any ERC-20 token by entering the contract address
- **Popular Token Pairs**: Quick selection of popular tokens (WMATIC, USDC, USDT, WETH, DAI)
- **Multiple Fee Tiers**: Choose from 0.05%, 0.3%, or 1% fee tiers based on pair volatility
- **Pool Creation**: Automatically create new pools if they don't exist
- **Real-time Balances**: View token balances in real-time
- **Commission System**: Built-in commission system (0.001 POL per transaction)
- **Responsive Design**: Beautiful, dark-themed UI that works on all devices
- **Polygon Network**: Low fees and fast transactions

## 🚀 Getting Started

### Prerequisites

- Node.js 16.x or later
- npm or yarn
- MetaMask or compatible Web3 wallet
- Polygon network setup in your wallet

### Installation

1. **Clone the repository**

   ```bash
   git clone <repository-url>
   cd uniswap-liquidity-creator
   ```

2. **Install dependencies**

   ```bash
   npm install
   # or
   yarn install
   ```

3. **Set up environment variables**

   Create a `.env.local` file in the root directory and add the following variables:

   ```env
   # Wallet Connect
   NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID=your_wallet_connect_project_id

   # Network Configuration
   NEXT_PUBLIC_RPC_URL=https://polygon-rpc.com
   NEXT_PUBLIC_CHAIN_ID=137

   # Commission Settings
   NEXT_PUBLIC_COMMISSION_AMOUNT=0.001
   NEXT_PUBLIC_COMMISSION_RECIPIENT=0xb2c822A8f05Ed6d0aD8C62aaa952Cc8249733DB4

   # Uniswap V3 Addresses (Polygon)
   NEXT_PUBLIC_UNISWAP_V3_FACTORY=0x1F98431c8aD98523631AE4a59f267346ea31F984
   NEXT_PUBLIC_UNISWAP_V3_POSITION_MANAGER=0xC36442b4a4522E871399CD717aBDD847Ab11FE88
   NEXT_PUBLIC_UNISWAP_V3_ROUTER=0xE592427A0AEce92De3Edee1F18E0157C05861564

   # Popular Tokens (Polygon)
   NEXT_PUBLIC_WMATIC_ADDRESS=0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270
   NEXT_PUBLIC_USDC_ADDRESS=0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174
   NEXT_PUBLIC_USDT_ADDRESS=0xc2132D05D31c914a87C6611C10748AEb04B58e8F
   NEXT_PUBLIC_WETH_ADDRESS=0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619
   NEXT_PUBLIC_DAI_ADDRESS=0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063
   ```

4. **Run the development server**

   ```bash
   npm run dev
   # or
   yarn dev
   ```

5. **Open your browser**

   Navigate to [http://localhost:3000](http://localhost:3000) to see the application.

## 📁 Project Structure

```
├── components/           # React components
│   ├── Header.js        # Header with wallet connection
│   ├── Footer.js        # Footer with links and info
│   ├── Icons.js         # SVG icon components
│   ├── TokenSelector.js # Token selection modal
│   └── LiquidityForm.js # Main liquidity form
├── constants/           # App constants and configurations
│   └── index.js         # Contract addresses, ABIs, tokens
├── hooks/              # Custom React hooks
│   ├── useTokens.js    # Token management
│   └── useLiquidity.js # Liquidity operations
├── pages/              # Next.js pages
│   ├── _app.js         # App configuration
│   ├── _document.js    # Document configuration
│   └── index.js        # Main page
├── styles/             # CSS styles
│   └── globals.css     # Global styles and Tailwind
├── utils/              # Utility functions
│   └── helpers.js      # Helper functions
├── wagmi.js            # Wagmi configuration
└── next.config.js      # Next.js configuration
```

## 🔧 Configuration

### Environment Variables

All important variables are stored in environment variables for easy configuration:

- **Network Settings**: Chain ID, RPC URL
- **Commission Settings**: Amount and recipient address
- **Contract Addresses**: Uniswap V3 contracts
- **Token Addresses**: Popular token addresses

### Customization

To customize the application for different networks or settings:

1. Update the environment variables in `.env.local`
2. Modify the `wagmi.js` configuration for different chains
3. Update contract addresses in `constants/index.js` if needed

## 💻 Usage

### For Users

1. **Connect Wallet**: Click "Connect Wallet" and connect your Web3 wallet
2. **Select Tokens**: Choose your token pair (custom or popular tokens)
3. **Enter Amounts**: Specify the amounts for each token
4. **Select Fee Tier**: Choose appropriate fee tier (0.05%, 0.3%, or 1%)
5. **Create Pool**: Pay the commission and create your liquidity pool

### For Developers

The application is built with modularity in mind:

- **Components**: Reusable UI components with props
- **Hooks**: Custom hooks for state management and Web3 interactions
- **Utils**: Helper functions for formatting and calculations
- **Constants**: Centralized configuration

## 🛠️ Built With

- **Next.js 13.2.4** - React framework
- **Tailwind CSS** - Utility-first CSS framework
- **Wagmi** - React hooks for Ethereum
- **RainbowKit** - Wallet connection library
- **Ethers.js** - Ethereum library
- **React Hot Toast** - Toast notifications
- **Uniswap V3 SDK** - Uniswap integration

## 🔐 Security Features

- **Input Validation**: All user inputs are validated
- **Transaction Safety**: Slippage protection and deadline management
- **Error Handling**: Comprehensive error handling and user feedback
- **Security Headers**: CSP and other security headers configured

## 🚀 Deployment

### Build for Production

```bash
npm run build
# or
yarn build
```

### Start Production Server

```bash
npm start
# or
yarn start
```

### Environment Setup

Make sure to set all environment variables in your deployment platform:

- Vercel: Add variables in Project Settings → Environment Variables
- Netlify: Add variables in Site Settings → Environment Variables
- Railway/Heroku: Set config vars in dashboard

## 📝 License

This project is open source and available under the [MIT License](LICENSE).

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📞 Support

If you have any questions or need help with the application:

- Create an issue in the repository
- Check the [Uniswap V3 Documentation](https://docs.uniswap.org/)
- Review the [Polygon Documentation](https://docs.polygon.technology/)

## ⚠️ Disclaimer

This application is for educational and development purposes. Always review smart contract interactions and understand the risks involved with DeFi protocols. Test thoroughly on testnets before using on mainnet.

---

**Built with ❤️ for the DeFi community**
# uniswapliquiditycreator
