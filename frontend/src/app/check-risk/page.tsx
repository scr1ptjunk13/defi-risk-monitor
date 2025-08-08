'use client';

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';

export default function CheckRiskPage() {
  const router = useRouter();
  const [walletAddress, setWalletAddress] = useState('');
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState(0);
  const [scanMessage, setScanMessage] = useState('');
  const [hasMetaMask, setHasMetaMask] = useState(false);

  // Check for MetaMask on component mount
  useEffect(() => {
    if (typeof window !== 'undefined' && (window as any).ethereum) {
      setHasMetaMask(true);
    }
  }, []);

  const connectMetaMask = async () => {
    try {
      setIsScanning(true);
      setScanProgress(0);
      
      const progressSteps = [
        { progress: 20, message: 'Connecting to wallet...' },
        { progress: 40, message: 'Scanning DeFi positions...' },
        { progress: 60, message: 'Analyzing risk factors...' },
        { progress: 80, message: 'Calculating portfolio metrics...' },
        { progress: 100, message: 'Analysis complete!' }
      ];
      
      for (const step of progressSteps) {
        await new Promise(resolve => setTimeout(resolve, 800));
        setScanProgress(step.progress);
        setScanMessage(step.message);
      }
      
      if (!(window as any).ethereum) {
        alert('Please install MetaMask to connect your wallet');
        setIsScanning(false);
        return;
      }

      // Request account access
      const accounts = await (window as any).ethereum.request({
        method: 'eth_requestAccounts',
      });

      const address = accounts[0];
      setWalletAddress(address);
      
      // Store wallet connection
      localStorage.setItem('connectedWallet', address);
      
      // Navigate to dashboard
      router.push('/dashboard');
    } catch (error) {
      console.error('Failed to connect MetaMask:', error);
      alert('Failed to connect wallet. Please try again.');
      setIsScanning(false);
    }
  };

  const analyzeManualAddress = async () => {
    if (!walletAddress.trim()) {
      alert('Please enter a wallet address or ENS name');
      return;
    }

    try {
      setIsScanning(true);
      
      // Simulate address validation and scanning
      await new Promise(resolve => setTimeout(resolve, 4000));
      
      localStorage.setItem('analyzedWallet', walletAddress);
      localStorage.setItem('connectedWallet', walletAddress); // Set as connected for dashboard
      
      // Navigate to dashboard
      router.push('/dashboard');
    } catch (error) {
      console.error('Failed to analyze address:', error);
      alert('Failed to analyze wallet. Please check the address and try again.');
      setIsScanning(false);
    }
  };

  const showDemo = async () => {
    try {
      setIsScanning(true);
      
      // Simulate loading demo data
      await new Promise(resolve => setTimeout(resolve, 2000));
      
      localStorage.setItem('demoMode', 'true');
      localStorage.setItem('connectedWallet', 'demo'); // Set demo wallet for dashboard
      
      // Navigate to dashboard
      router.push('/dashboard');
    } catch (error) {
      console.error('Failed to load demo:', error);
      setIsScanning(false);
    }
  };

  // Loading state for wallet scanning
  if (isScanning) {
    return (
      <div className="min-h-screen bg-black flex items-center justify-center">
        <div className="text-center max-w-md">
          <div className="animate-spin rounded-full h-16 w-16 border-b-2 border-red-400 mx-auto mb-6"></div>
          <h2 className="text-2xl font-bold text-white mb-4">
            {walletAddress ? 'Analyzing Your DeFi Positions...' : 'Scanning Your Wallet...'}
          </h2>
          <p className="text-gray-300 mb-6">
            {walletAddress ? 
              'Checking Uniswap, Aave, Compound, Curve, and 50+ protocols...' :
              'Connecting to your wallet securely...'
            }
          </p>
          <div className="bg-gradient-to-br from-gray-900/60 to-gray-800/40 border border-gray-700/50 rounded-xl p-4">
            <div className="text-sm text-gray-400 space-y-2">
              <div className="flex items-center">
                <div className="w-2 h-2 bg-green-400 rounded-full mr-3"></div>
                <span>Scanning liquidity pools...</span>
              </div>
              <div className="flex items-center">
                <div className="w-2 h-2 bg-yellow-400 rounded-full mr-3 animate-pulse"></div>
                <span>Analyzing lending positions...</span>
              </div>
              <div className="flex items-center">
                <div className="w-2 h-2 bg-gray-400 rounded-full mr-3"></div>
                <span>Calculating risk metrics...</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-black">
      {/* Header */}
      <header className="bg-black/90 backdrop-blur-xl border-b border-gray-800/50">
        <div className="max-w-6xl mx-auto px-4 py-6">
          <div className="text-center">
            <h1 className="text-4xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
              DeFi Risk Monitor
            </h1>
          </div>
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-16">
        <div className="text-center mb-12">
          <h2 className="text-3xl font-bold text-white mb-4">
            Choose How to Check Your Risk
          </h2>
          <p className="text-gray-300">
            We'll analyze your DeFi positions and show you exactly what risks you're facing
          </p>
        </div>

        <div className="space-y-6">
          {/* MetaMask Connection */}
          {hasMetaMask && (
            <div className="bg-gradient-to-br from-orange-900/30 to-orange-800/20 border border-orange-500/30 rounded-2xl p-6 backdrop-blur-sm">
              <div className="flex items-center mb-4">
                <div className="text-3xl mr-4">🦊</div>
                <div>
                  <h3 className="text-xl font-bold text-white">Connect MetaMask</h3>
                  <p className="text-gray-300 text-sm">Instant analysis of all your positions</p>
                </div>
              </div>
              <button 
                onClick={connectMetaMask}
                className="w-full bg-gradient-to-r from-orange-600 to-orange-700 text-white rounded-xl py-4 px-6 font-semibold hover:from-orange-500 hover:to-orange-600 transition-all duration-300 transform hover:scale-105 shadow-lg"
              >
                Connect MetaMask Wallet
              </button>
              <p className="text-xs text-gray-400 mt-2">✅ Most secure • ✅ Instant results • ✅ All protocols</p>
            </div>
          )}

          {/* Manual Address Input */}
          <div className="bg-gradient-to-br from-blue-900/30 to-blue-800/20 border border-blue-500/30 rounded-2xl p-6 backdrop-blur-sm">
            <div className="flex items-center mb-4">
              <div className="text-3xl mr-4">📝</div>
              <div>
                <h3 className="text-xl font-bold text-white">Enter Wallet Address</h3>
                <p className="text-gray-300 text-sm">Paste your address or ENS name</p>
              </div>
            </div>
            <input
              type="text"
              value={walletAddress}
              onChange={(e) => setWalletAddress(e.target.value)}
              placeholder="0x1234...abcd or myname.eth"
              className="w-full bg-gray-800/50 border border-gray-600/50 rounded-xl py-3 px-4 text-white placeholder-gray-400 mb-4 focus:border-blue-500/50 focus:outline-none"
            />
            <button 
              onClick={analyzeManualAddress}
              className="w-full bg-gradient-to-r from-blue-600 to-blue-700 text-white rounded-xl py-4 px-6 font-semibold hover:from-blue-500 hover:to-blue-600 transition-all duration-300 transform hover:scale-105 shadow-lg"
            >
              Analyze This Wallet
            </button>
            <p className="text-xs text-gray-400 mt-2">✅ Works on any device • ✅ No wallet needed • ✅ ENS supported</p>
          </div>

          {/* Demo Mode */}
          <div className="bg-gradient-to-br from-green-900/30 to-green-800/20 border border-green-500/30 rounded-2xl p-6 backdrop-blur-sm">
            <div className="flex items-center mb-4">
              <div className="text-3xl mr-4">🎮</div>
              <div>
                <h3 className="text-xl font-bold text-white">See Demo First</h3>
                <p className="text-gray-300 text-sm">Explore with sample portfolio</p>
              </div>
            </div>
            <button 
              onClick={showDemo}
              className="w-full bg-gradient-to-r from-green-600 to-green-700 text-white rounded-xl py-4 px-6 font-semibold hover:from-green-500 hover:to-green-600 transition-all duration-300 transform hover:scale-105 shadow-lg"
            >
              View Sample Analysis
            </button>
            <p className="text-xs text-gray-400 mt-2">✅ No commitment • ✅ See features • ✅ Instant preview</p>
          </div>
        </div>

        {/* Back Button */}
        <div className="text-center mt-8">
          <button 
            onClick={() => router.back()}
            className="text-gray-400 hover:text-white transition-colors"
          >
            ← Back to main page
          </button>
        </div>
      </main>
    </div>
  );
}
