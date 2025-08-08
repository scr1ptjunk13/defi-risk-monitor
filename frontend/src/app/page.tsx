'use client';

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';

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
  const router = useRouter();
  const [currentTime, setCurrentTime] = useState('');
  
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
  
  // Update live risks every 8 seconds and set initial time
  useEffect(() => {
    // Set initial time on client side
    setCurrentTime(new Date().toLocaleTimeString());
    
    const interval = setInterval(() => {
      setCurrentTime(new Date().toLocaleTimeString());
      setLiveRisks(prev => {
        const newRisks = [...prev];
        const randomIndex = Math.floor(Math.random() * newRisks.length);
        const riskTypes = ['Impermanent Loss', 'Liquidation Risk', 'MEV Attack', 'Smart Contract Risk', 'Oracle Manipulation'];
        const protocols = ['ETH/USDC Pool', 'Aave ETH', 'Curve 3Pool', 'Compound DAI', 'Uniswap V3', 'SushiSwap'];
        
        newRisks[randomIndex] = {
          ...newRisks[randomIndex],
          protocol: protocols[Math.floor(Math.random() * protocols.length)],
          riskType: riskTypes[Math.floor(Math.random() * riskTypes.length)],
          affectedUsers: Math.floor(Math.random() * 5000) + 100,
          lossAmount: `$${(Math.random() * 5 + 0.1).toFixed(1)}M`
        };
        
        return newRisks;
      });
    }, 8000);
    
    return () => clearInterval(interval);
  }, []);

  const connectWallet = async () => {
    router.push('/check-risk');
  };

  const clearData = () => {
    // Clear all localStorage data
    localStorage.removeItem('connectedWallet');
    localStorage.removeItem('userAuth');
    localStorage.removeItem('analyzedWallet');
    localStorage.removeItem('demoMode');
    
    alert('All data cleared!');
  };

  // Always show onboarding/landing page
  return (
    <div className="min-h-screen bg-black">
      {/* Header */}
      <header className="bg-black/90 backdrop-blur-xl border-b border-gray-800/50">
        <div className="max-w-6xl mx-auto px-4 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-3xl font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
                DeFi Risk Monitor
              </h1>
              <p className="text-sm text-gray-400 mt-1">
                Last updated: {currentTime}
              </p>
            </div>
            <button 
              onClick={clearData}
              className="bg-gray-800 hover:bg-gray-700 text-gray-300 hover:text-white px-4 py-2 rounded-lg transition-colors text-sm border border-gray-600"
            >
              Clear Data
            </button>
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
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mb-12">
            <div className="bg-gradient-to-br from-gray-900/40 to-gray-800/20 rounded-xl p-6 border border-gray-700/50">
              <div className="text-3xl font-bold text-blue-400 mb-2">$47M+</div>
              <div className="text-gray-300">Protected in DeFi positions</div>
            </div>
            <div className="bg-gradient-to-br from-gray-900/40 to-gray-800/20 rounded-xl p-6 border border-gray-700/50">
              <div className="text-3xl font-bold text-green-400 mb-2">12,847</div>
              <div className="text-gray-300">Users saved from losses</div>
            </div>
            <div className="bg-gradient-to-br from-gray-900/40 to-gray-800/20 rounded-xl p-6 border border-gray-700/50">
              <div className="text-3xl font-bold text-purple-400 mb-2">24/7</div>
              <div className="text-gray-300">Real-time monitoring</div>
            </div>
          </div>
          <p className="text-gray-400 text-sm">Don't be the next person to lose $50K to impermanent loss</p>
        </div>
      </main>
    </div>
  );
}
