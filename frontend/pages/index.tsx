import { useState, useEffect } from "react";
import Head from "next/head";
import { useAccount } from "wagmi";
import Header from "../components/Header";
import Footer from "../components/Footer";
import LiquidityForm from "../components/LiquidityForm";
import RiskDashboard from "../components/RiskDashboard";
import { InfoIcon, CheckIcon } from "../components/Icons";

export default function Home() {
  const [mounted, setMounted] = useState(false);
  const [activeTab, setActiveTab] = useState<'liquidity' | 'dashboard'>('liquidity');
  const { isConnected } = useAccount();

  // Fix hydration error
  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return null;
  }

  const features = [
    {
      icon: <CheckIcon className="w-5 h-5 text-green-400" />,
      title: "Custom Token Support",
      description:
        "Add liquidity for your newly created tokens by simply entering the contract address.",
    },
    {
      icon: <CheckIcon className="w-5 h-5 text-green-400" />,
      title: "Popular Token Pairs",
      description: "Quick selection of popular tokens like WPOL, NATIVE TOKEN.",
    },
    {
      icon: <CheckIcon className="w-5 h-5 text-green-400" />,
      title: "Multiple Fee Tiers",
      description:
        "Choose from 0.05%, 0.3%, or 1% fee tiers based on your token pair volatility.",
    },
    {
      icon: <CheckIcon className="w-5 h-5 text-green-400" />,
      title: "Polygon Network",
      description:
        "Low fees and fast transactions on Polygon network for optimal user experience.",
    },
    {
      icon: <CheckIcon className="w-5 h-5 text-green-400" />,
      title: "Pool Creation",
      description:
        "Create new pools if they don't exist, then add liquidity in a single transaction.",
    },
    {
      icon: <CheckIcon className="w-5 h-5 text-green-400" />,
      title: "Real-time Balance",
      description:
        "View your token balances in real-time to make informed liquidity decisions.",
    },
  ];

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
      <Head>
        <title>Uniswap Liquidity Creator - Polygon Network</title>
        <meta
          name="description"
          content="Create liquidity pools for your tokens on Uniswap V3 with ease. Support for custom tokens and popular pairs on Polygon network."
        />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <Header />

      <main className="flex-1">
        {/* Hero Section */}
        <section className="py-12 px-4 sm:px-6 lg:px-8">
          <div className="max-w-7xl mx-auto">
            <div className="text-center mb-12">
              <h1 className="text-4xl md:text-6xl font-bold text-white mb-6">
                Create Liquidity Pools
                <span className="block text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-purple-600">
                  Launch Your Token
                </span>
              </h1>
              <p className="text-xl text-gray-300 max-w-3xl mx-auto mb-8">
                Add liquidity to Uniswap V3 on Polygon network. Support for
                custom tokens and popular pairs with low fees and fast
                transactions.
              </p>

              {!isConnected && (
                <div className="bg-blue-900/20 border border-blue-500/30 rounded-lg p-4 max-w-md mx-auto mb-8">
                  <div className="flex items-center space-x-2">
                    <InfoIcon className="w-5 h-5 text-blue-400 flex-shrink-0" />
                    <p className="text-blue-200 text-sm">
                      Connect your wallet to get started with creating liquidity
                      pools
                    </p>
                  </div>
                </div>
              )}
            </div>

            {/* Tab Navigation */}
            <div className="flex justify-center mb-8">
              <div className="bg-gray-800/50 rounded-lg p-1 border border-gray-700">
                <button
                  onClick={() => setActiveTab('liquidity')}
                  className={`px-6 py-3 rounded-md font-medium transition-all ${
                    activeTab === 'liquidity'
                      ? 'bg-blue-600 text-white shadow-lg'
                      : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
                  }`}
                >
                  🚀 Add Liquidity
                </button>
                <button
                  onClick={() => setActiveTab('dashboard')}
                  className={`px-6 py-3 rounded-md font-medium transition-all ${
                    activeTab === 'dashboard'
                      ? 'bg-blue-600 text-white shadow-lg'
                      : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
                  }`}
                >
                  📊 Risk Dashboard
                </button>
              </div>
            </div>

            {/* Tab Content */}
            {activeTab === 'liquidity' ? (
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-start">
                {/* Liquidity Form */}
                <div className="order-2 lg:order-1">
                  <LiquidityForm />
                </div>

                {/* Features */}
                <div className="order-1 lg:order-2">
                <div className="bg-gray-900/50 rounded-2xl p-8 border border-gray-700">
                  <h2 className="text-2xl font-bold text-white mb-6">
                    Platform Features
                  </h2>
                  <div className="space-y-6">
                    {features.map((feature, index) => (
                      <div key={index} className="flex items-start space-x-4">
                        <div className="flex-shrink-0">{feature.icon}</div>
                        <div>
                          <h3 className="text-lg font-semibold text-white mb-2">
                            {feature.title}
                          </h3>
                          <p className="text-gray-400">{feature.description}</p>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Stats */}
                <div className="mt-8 grid grid-cols-3 gap-4">
                  <div className="bg-gray-900/50 rounded-xl p-4 text-center border border-gray-700">
                    <div className="text-2xl font-bold text-blue-400">
                      0.001
                    </div>
                    <div className="text-sm text-gray-400">POL Fee</div>
                  </div>
                  <div className="bg-gray-900/50 rounded-xl p-4 text-center border border-gray-700">
                    <div className="text-2xl font-bold text-green-400">3</div>
                    <div className="text-sm text-gray-400">Fee Tiers</div>
                  </div>
                  <div className="bg-gray-900/50 rounded-xl p-4 text-center border border-gray-700">
                    <div className="text-2xl font-bold text-purple-400">V3</div>
                    <div className="text-sm text-gray-400">Uniswap</div>
                  </div>
                </div>
              </div>
            </div>
            ) : (
              /* Risk Dashboard Tab */
              <div className="max-w-6xl mx-auto">
                <RiskDashboard className="w-full" />
              </div>
            )}
          </div>
        </section>

        {/* How It Works */}
        <section className="py-16 px-4 sm:px-6 lg:px-8 bg-gray-800/50">
          <div className="max-w-7xl mx-auto">
            <div className="text-center mb-12">
              <h2 className="text-3xl font-bold text-white mb-4">
                How It Works
              </h2>
              <p className="text-gray-300 max-w-2xl mx-auto">
                Simple steps to create liquidity pools and launch your token on
                Uniswap
              </p>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-4 gap-8">
              {[
                {
                  step: "1",
                  title: "Connect Wallet",
                  description: "Connect your wallet to the Polygon network",
                },
                {
                  step: "2",
                  title: "Select Tokens",
                  description: "Choose your token pair and enter amounts",
                },
                {
                  step: "3",
                  title: "Set Parameters",
                  description: "Select fee tier and price range settings",
                },
                {
                  step: "4",
                  title: "Create Pool",
                  description: "Pay fee and create your liquidity pool",
                },
              ].map((item, index) => (
                <div key={index} className="text-center">
                  <div className="w-12 h-12 bg-gradient-to-r from-blue-500 to-purple-600 rounded-full flex items-center justify-center text-white font-bold text-lg mx-auto mb-4">
                    {item.step}
                  </div>
                  <h3 className="text-lg font-semibold text-white mb-2">
                    {item.title}
                  </h3>
                  <p className="text-gray-400">{item.description}</p>
                </div>
              ))}
            </div>
          </div>
        </section>
      </main>

      <Footer />
    </div>
  );
}
