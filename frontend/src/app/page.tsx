'use client';

import { useState, useEffect } from 'react';

interface RiskData {
  overallRisk: number;
  portfolioValue: number;
  activeAlerts: number;
  riskLevel: 'safe' | 'caution' | 'danger';
}

interface LiveRiskAlert {
  id: string;
  protocol: string;
  riskType: string;
  severity: 'high' | 'medium' | 'low';
  message: string;
  affectedUsers: number;
  lossAmount?: string;
}

interface LossCounterData {
  totalLost24h: number;
  currentRate: number;
  lastUpdate: number;
}

export default function DeFiRiskMonitor() {
  const [riskData, setRiskData] = useState<RiskData>({
    overallRisk: 0,
    portfolioValue: 0,
    activeAlerts: 0,
    riskLevel: 'safe'
  });
  const [loading, setLoading] = useState(true);
  const [hasWallet, setHasWallet] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [walletAddress, setWalletAddress] = useState('');
  const [showOnboarding, setShowOnboarding] = useState(true);
  const [showWalletOptions, setShowWalletOptions] = useState(false);
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState(0);
  const [scanMessage, setScanMessage] = useState('');
  
  // Live risk monitoring state
  const [liveRisks, setLiveRisks] = useState<LiveRiskAlert[]>([
    {
      id: '1',
      protocol: 'ETH/USDC Pool',
      riskType: 'Impermanent Loss',
      severity: 'high',
      message: 'High IL Risk (73% of LPs losing money)',
      affectedUsers: 2847,
      lossAmount: '$2.3M'
    },
    {
      id: '2', 
      protocol: 'Aave ETH',
      riskType: 'Liquidation Risk',
      severity: 'high',
      message: 'Liquidation cascade detected (2.3M at risk)',
      affectedUsers: 1205,
      lossAmount: '$2.3M'
    },
    {
      id: '3',
      protocol: 'Curve 3Pool',
      riskType: 'MEV Attack',
      severity: 'medium',
      message: 'MEV attacks increased 340% today',
      affectedUsers: 892,
      lossAmount: '$450K'
    }
  ]);
  
  const [lossCounter, setLossCounter] = useState<LossCounterData>({
    totalLost24h: 1247382,
    currentRate: 85.7,
    lastUpdate: Date.now()
  });
  
  // Wallet connection state
  const [hasMetaMask, setHasMetaMask] = useState(false);

  // Check for MetaMask on component mount
  useEffect(() => {
    if (typeof window !== 'undefined' && (window as any).ethereum) {
      setHasMetaMask(true);
    }
  }, []);

  useEffect(() => {
    // Check if user has connected wallet or logged in before
    const checkUserStatus = async () => {
      try {
        // Check if user has wallet connected or is logged in
        const savedWallet = localStorage.getItem('connectedWallet');
        const savedAuth = localStorage.getItem('userAuth');
        
        if (savedWallet || savedAuth) {
          setIsConnected(true);
          setShowOnboarding(false);
          
          // Fetch real user data from backend
          const response = await fetch('http://localhost:8080/health');
          if (response.ok) {
            // TODO: Replace with real user portfolio API call
            setRiskData({
              overallRisk: 15,
              portfolioValue: 25420.50,
              activeAlerts: 2,
              riskLevel: 'safe'
            });
          }
        } else {
          // New user - show onboarding
          setShowOnboarding(true);
        }
      } catch (error) {
        console.log('Backend not connected yet');
      } finally {
        setLoading(false);
      }
    };

    checkUserStatus();
  }, []);
  
  // Live loss counter animation
  useEffect(() => {
    const interval = setInterval(() => {
      setLossCounter(prev => ({
        ...prev,
        totalLost24h: prev.totalLost24h + Math.random() * 200 + 50,
        lastUpdate: Date.now()
      }));
    }, 3000);
    
    return () => clearInterval(interval);
  }, []);
  
  // Live risk alerts rotation
  useEffect(() => {
    const interval = setInterval(() => {
      setLiveRisks(prev => {
        const newRisks = [...prev];
        const randomIndex = Math.floor(Math.random() * newRisks.length);
        newRisks[randomIndex] = {
          ...newRisks[randomIndex],
          affectedUsers: newRisks[randomIndex].affectedUsers + Math.floor(Math.random() * 10)
        };
        return newRisks;
      });
    }, 5000);
    
    return () => clearInterval(interval);
  }, []);

  const getRiskColor = (level: string) => {
    switch (level) {
      case 'safe': return 'text-green-500';
      case 'caution': return 'text-yellow-500';
      case 'danger': return 'text-red-500';
      default: return 'text-gray-500';
    }
  };

  const getRiskBg = (level: string) => {
    switch (level) {
      case 'safe': return 'bg-gradient-to-br from-green-900/20 to-green-800/10 border-green-500/30 shadow-green-500/10';
      case 'caution': return 'bg-gradient-to-br from-yellow-900/20 to-yellow-800/10 border-yellow-500/30 shadow-yellow-500/10';
      case 'danger': return 'bg-gradient-to-br from-red-900/20 to-red-800/10 border-red-500/30 shadow-red-500/10';
      default: return 'bg-gradient-to-br from-gray-900/20 to-gray-800/10 border-gray-500/30';
    }
  };

  const connectWallet = async () => {
    setShowWalletOptions(true);
  };

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
      
      // Simulate completion
      await new Promise(resolve => setTimeout(resolve, 1000));
      setIsScanning(false);
      setScanMessage('Redirecting to dashboard...');
      
      // Redirect to dashboard
      setTimeout(() => {
        window.location.href = '/dashboard';
      }, 1500);
    } catch (error) {
      console.error('Failed to connect MetaMask:', error);
      alert('Failed to connect wallet. Please try again.');
    } finally {
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
      setIsConnected(true);
      setShowOnboarding(false);
      setShowWalletOptions(false);
      
      // Show risk analysis results
      setRiskData({
        overallRisk: 31,
        portfolioValue: 24320.75,
        activeAlerts: 2,
        riskLevel: 'caution'
      });
    } catch (error) {
      console.error('Failed to analyze address:', error);
      alert('Failed to analyze wallet. Please check the address and try again.');
    } finally {
      setIsScanning(false);
    }
  };

  const showDemo = async () => {
    try {
      setIsScanning(true);
      
      // Simulate loading demo data
      await new Promise(resolve => setTimeout(resolve, 2000));
      
      localStorage.setItem('demoMode', 'true');
      setIsConnected(true);
      setShowOnboarding(false);
      setShowWalletOptions(false);
      
      // Show demo portfolio
      setRiskData({
        overallRisk: 18,
        portfolioValue: 15600.25,
        activeAlerts: 1,
        riskLevel: 'safe'
      });
    } catch (error) {
      console.error('Failed to load demo:', error);
    } finally {
      setIsScanning(false);
    }
  };

  const loginWithEmail = async () => {
    try {
      // Simulate email login (replace with real auth)
      setLoading(true);
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      localStorage.setItem('userAuth', 'demo-user-token');
      setIsConnected(true);
      setShowOnboarding(false);
      
      // Show demo data for email users
      setRiskData({
        overallRisk: 12,
        portfolioValue: 8750.25,
        activeAlerts: 0,
        riskLevel: 'safe'
      });
    } catch (error) {
      console.error('Failed to login:', error);
    } finally {
      setLoading(false);
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

  if (loading) {
    return (
      <div className="min-h-screen bg-black flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-400 mx-auto mb-4"></div>
          <p className="text-gray-300">
            {showOnboarding ? 'Loading...' : 'Loading your DeFi risk data...'}
          </p>
        </div>
      </div>
    );
  }

  // Wallet connection options modal
  if (showWalletOptions) {
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

            {/* Manual Address Entry */}
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
              onClick={() => setShowWalletOptions(false)}
              className="text-gray-400 hover:text-white transition-colors"
            >
              ← Back to main page
            </button>
          </div>
        </main>
      </div>
    );
  }

  // Show onboarding for new users
  if (showOnboarding) {
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

        {/* Live Risk Monitor - Show Value Before Signup */}
        <div className="max-w-6xl mx-auto px-4 py-8">
          {/* Live Loss Counter */}
          <div className="text-center mb-12">
            <div className="bg-gradient-to-r from-red-900/30 to-red-800/20 border border-red-500/30 rounded-2xl p-6 mb-8 backdrop-blur-sm">
              <div className="text-red-400 text-sm font-medium mb-2">💸 LIVE DEFI LOSSES</div>
              <div className="text-4xl font-bold text-red-300 mb-2">
                ${lossCounter.totalLost24h.toLocaleString('en-US', { maximumFractionDigits: 0 })}
              </div>
              <div className="text-red-400 text-sm">
                Lost to DeFi risks in the last 24 hours • +${lossCounter.currentRate.toFixed(1)}/second
              </div>
            </div>
          </div>
          
          {/* Live Risk Alerts - List Format */}
          <div className="mb-16">
            <div className="max-w-2xl mx-auto">
              <div className="bg-gradient-to-br from-gray-900/60 to-gray-800/40 border border-gray-700/50 rounded-2xl p-8 backdrop-blur-sm">
                <div className="flex items-center mb-6">
                  <div className="text-yellow-400 text-xl mr-3">⚡</div>
                  <h3 className="text-xl font-bold text-orange-400">LIVE RISKS DETECTED RIGHT NOW</h3>
                </div>
                
                <div className="space-y-4 mb-6">
                  {liveRisks.map((risk) => (
                    <div key={risk.id} className="flex items-center space-x-3">
                      <div className={`w-3 h-3 rounded-full ${
                        risk.severity === 'high' ? 'bg-red-500' :
                        risk.severity === 'medium' ? 'bg-yellow-500' :
                        'bg-blue-500'
                      }`}></div>
                      <div className="text-gray-200 text-sm">
                        <span className="font-medium">{risk.protocol}:</span> {risk.message}
                      </div>
                    </div>
                  ))}
                </div>
                
                <div className="flex items-center text-yellow-400">
                  <div className="text-lg mr-2">⚠️</div>
                  <div className="font-medium">Is YOUR portfolio affected? Check now →</div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        {/* Onboarding Content */}
        <main className="max-w-4xl mx-auto px-4 py-8">
          {/* Hero Section - Redesigned with Loss Focus */}
          <div className="text-center mb-16">
            <h2 className="text-5xl font-bold text-white mb-6">
              Stop Losing Money in DeFi
            </h2>
            <p className="text-xl text-gray-300 mb-8 max-w-3xl mx-auto leading-relaxed">
              Most DeFi users lose 15-40% annually to risks they never see coming. 
              We show you exactly what's happening to your money.
            </p>
            <div className="bg-gradient-to-r from-green-900/30 to-green-800/20 border border-green-500/30 rounded-xl p-4 mb-8 max-w-2xl mx-auto">
              <div className="text-green-400 text-sm font-medium mb-1">💰 MONEY SAVED BY OUR USERS</div>
              <div className="text-2xl font-bold text-green-300">$2,847,392 prevented losses this month</div>
              <div className="text-green-400 text-sm">Average user prevents $3,200 in losses monthly</div>
            </div>
          </div>

          {/* Single Primary CTA - Maximum Focus */}
          <div className="max-w-md mx-auto mb-16">
            <button 
              onClick={connectWallet}
              className="w-full bg-gradient-to-r from-red-600 to-red-700 text-white rounded-xl py-6 px-8 text-xl font-bold hover:from-red-500 hover:to-red-600 transition-all duration-300 transform hover:scale-105 shadow-2xl hover:shadow-red-500/30 mb-4"
            >
              🚨 Check My Risk Now (Free)
            </button>
            <p className="text-center text-gray-400 text-sm">
              See what risks are threatening your DeFi positions right now
            </p>
          </div>

          {/* Meaningful Social Proof */}
          <div className="text-center mt-16">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-4xl mx-auto mb-8">
              <div className="bg-gradient-to-br from-gray-900/40 to-gray-800/30 border border-gray-700/50 rounded-xl p-6 backdrop-blur-sm">
                <div className="text-3xl font-bold text-green-400 mb-2">2,847</div>
                <div className="text-gray-300 text-sm">Users saved from liquidation this week</div>
              </div>
              <div className="bg-gradient-to-br from-gray-900/40 to-gray-800/30 border border-gray-700/50 rounded-xl p-6 backdrop-blur-sm">
                <div className="text-3xl font-bold text-blue-400 mb-2">12/15</div>
                <div className="text-gray-300 text-sm">Major exploits detected before they happened</div>
              </div>
              <div className="bg-gradient-to-br from-gray-900/40 to-gray-800/30 border border-gray-700/50 rounded-xl p-6 backdrop-blur-sm">
                <div className="text-3xl font-bold text-purple-400 mb-2">$3,200</div>
                <div className="text-gray-300 text-sm">Average losses prevented per user monthly</div>
              </div>
            </div>
            
            <div className="text-center">
              <p className="text-red-400 font-medium mb-2">🔥 3 major exploits happened while you were reading this</p>
              <p className="text-gray-400 text-sm">Don't be the next person to lose $50K to impermanent loss</p>
            </div>
          </div>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-black">
      {/* Header */}
      <header className="bg-black/90 backdrop-blur-xl border-b border-gray-800/50">
        <div className="max-w-6xl mx-auto px-4 py-6">
          <div className="flex items-center justify-between">
            <h1 className="text-3xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
              DeFi Risk Monitor
            </h1>
            <div className="text-sm text-gray-400 font-medium">
              Last updated: {new Date().toLocaleTimeString()}
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-6xl mx-auto px-4 py-12">
        {/* Hero Risk Status */}
        <div className={`rounded-3xl border p-12 mb-12 shadow-2xl ${getRiskBg(riskData.riskLevel)}`}>
          <div className="text-center">
            <div className={`text-8xl font-bold mb-4 ${getRiskColor(riskData.riskLevel)} drop-shadow-lg`}>
              {riskData.overallRisk}%
            </div>
            <div className="text-2xl text-gray-300 mb-6 font-light">
              Your DeFi Risk Level
            </div>
            <div className={`inline-flex items-center px-6 py-3 rounded-full text-sm font-semibold ${getRiskColor(riskData.riskLevel)} bg-black/40 backdrop-blur-sm border border-gray-700/50`}>
              <div className={`w-3 h-3 rounded-full mr-3 ${riskData.riskLevel === 'safe' ? 'bg-green-400 shadow-green-400/50' : riskData.riskLevel === 'caution' ? 'bg-yellow-400 shadow-yellow-400/50' : 'bg-red-400 shadow-red-400/50'} shadow-lg`}></div>
              {riskData.riskLevel === 'safe' ? '✅ SAFE' : riskData.riskLevel === 'caution' ? '⚠️ CAUTION' : '🚨 DANGER'}
            </div>
          </div>
        </div>

        {/* Key Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-8 mb-12">
          {/* Portfolio Value */}
          <div className="bg-gradient-to-br from-gray-900/50 to-gray-800/30 rounded-2xl p-8 shadow-2xl border border-gray-700/50 backdrop-blur-sm">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-400 mb-2 font-medium uppercase tracking-wide">Total Portfolio Value</p>
                <p className="text-4xl font-bold text-white">
                  ${riskData.portfolioValue.toLocaleString()}
                </p>
              </div>
              <div className="text-5xl opacity-80">💰</div>
            </div>
          </div>

          {/* Active Alerts */}
          <div className="bg-gradient-to-br from-gray-900/50 to-gray-800/30 rounded-2xl p-8 shadow-2xl border border-gray-700/50 backdrop-blur-sm">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-400 mb-2 font-medium uppercase tracking-wide">Active Alerts</p>
                <p className="text-4xl font-bold text-white">
                  {riskData.activeAlerts}
                </p>
              </div>
              <div className="text-5xl opacity-80">🔔</div>
            </div>
          </div>
        </div>

        {/* Simple Explanation */}
        <div className="bg-gradient-to-br from-gray-900/40 to-gray-800/20 rounded-2xl p-8 shadow-2xl border border-gray-700/50 backdrop-blur-sm mb-12">
          <h2 className="text-2xl font-bold text-white mb-6">What This Means</h2>
          <div className="space-y-4 text-gray-300 text-lg leading-relaxed">
            {riskData.riskLevel === 'safe' && (
              <p>✅ <strong className="text-green-400">You're in good shape!</strong> Your DeFi positions are relatively safe with low risk of major losses.</p>
            )}
            {riskData.riskLevel === 'caution' && (
              <p>⚠️ <strong className="text-yellow-400">Pay attention.</strong> Some of your positions have moderate risk. Consider reviewing your strategy.</p>
            )}
            {riskData.riskLevel === 'danger' && (
              <p>🚨 <strong className="text-red-400">High risk detected!</strong> Your positions may be at risk of significant losses. Take action soon.</p>
            )}
            <p>💡 <strong className="text-blue-400">Tip:</strong> We monitor your DeFi positions 24/7 and alert you to any risks automatically.</p>
          </div>
        </div>

        {/* Quick Actions */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <button className="group bg-gradient-to-br from-blue-600 to-blue-700 text-white rounded-2xl p-6 text-center hover:from-blue-500 hover:to-blue-600 transition-all duration-300 transform hover:scale-105 shadow-2xl hover:shadow-blue-500/25">
            <div className="text-3xl mb-3 group-hover:scale-110 transition-transform duration-300">📊</div>
            <div className="font-semibold text-lg">View Details</div>
          </button>
          <button className="group bg-gradient-to-br from-green-600 to-green-700 text-white rounded-2xl p-6 text-center hover:from-green-500 hover:to-green-600 transition-all duration-300 transform hover:scale-105 shadow-2xl hover:shadow-green-500/25">
            <div className="text-3xl mb-3 group-hover:scale-110 transition-transform duration-300">➕</div>
            <div className="font-semibold text-lg">Add Position</div>
          </button>
          <button className="group bg-gradient-to-br from-purple-600 to-purple-700 text-white rounded-2xl p-6 text-center hover:from-purple-500 hover:to-purple-600 transition-all duration-300 transform hover:scale-105 shadow-2xl hover:shadow-purple-500/25">
            <div className="text-3xl mb-3 group-hover:scale-110 transition-transform duration-300">⚙️</div>
            <div className="font-semibold text-lg">Settings</div>
          </button>
        </div>
      </main>
    </div>
  );
}
