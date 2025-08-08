'use client';

import React, { useState, useEffect, useRef } from 'react';

interface AIInsightsHubProps {
  userAddress: string;
  userTier: 'basic' | 'professional' | 'institutional' | 'enterprise';
}

interface ChatMessage {
  id: string;
  type: 'user' | 'ai';
  content: string;
  timestamp: number;
  suggestions?: string[];
}

interface RiskExplanation {
  factor: string;
  score: number;
  explanation: string;
  impact: string;
  recommendation: string;
}

interface AIRecommendation {
  id: string;
  type: 'optimization' | 'risk_reduction' | 'opportunity' | 'rebalancing';
  title: string;
  description: string;
  impact: number;
  confidence: number;
  timeframe: string;
  actions: string[];
}

const AIInsightsHub: React.FC<AIInsightsHubProps> = ({ userAddress, userTier }) => {
  const [activeTab, setActiveTab] = useState<'chat' | 'explanations' | 'recommendations' | 'predictions'>('chat');
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [currentMessage, setCurrentMessage] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [riskExplanations, setRiskExplanations] = useState<RiskExplanation[]>([]);
  const [recommendations, setRecommendations] = useState<AIRecommendation[]>([]);
  const chatEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Initialize with welcome message and mock data
    setChatMessages([
      {
        id: '1',
        type: 'ai',
        content: 'Hello! I\'m your AI risk analyst. I can explain your portfolio risks, suggest optimizations, and answer questions about your DeFi positions. How can I help you today?',
        timestamp: Date.now(),
        suggestions: [
          'Explain my highest risk position',
          'What are my optimization opportunities?',
          'How can I reduce MEV risk?',
          'Show me correlation risks'
        ]
      }
    ]);

    setRiskExplanations([
      {
        factor: 'MEV Risk',
        score: 82,
        explanation: 'Your ETH/USDC position on Uniswap V3 is highly vulnerable to MEV attacks due to its large size and active price range.',
        impact: 'Potential loss of 0.5-2% of position value through sandwich attacks and front-running.',
        recommendation: 'Consider using MEV protection services like Flashbots Protect or splitting large trades into smaller chunks.'
      },
      {
        factor: 'Liquidity Risk',
        score: 65,
        explanation: 'Several positions are in pools with declining liquidity, particularly during volatile market conditions.',
        impact: 'Increased slippage and difficulty exiting positions during market stress.',
        recommendation: 'Monitor pool depth regularly and consider migrating to more liquid alternatives.'
      },
      {
        factor: 'Protocol Risk',
        score: 45,
        explanation: 'Your protocol diversification is good, but some protocols have upcoming governance changes.',
        impact: 'Potential changes to fee structures or risk parameters.',
        recommendation: 'Stay informed about governance proposals and consider the impact on your positions.'
      }
    ]);

    setRecommendations([
      {
        id: '1',
        type: 'risk_reduction',
        title: 'Reduce MEV Exposure',
        description: 'Your ETH/USDC position is highly exposed to MEV attacks. Consider implementing protection strategies.',
        impact: 15.2,
        confidence: 87,
        timeframe: 'Immediate',
        actions: [
          'Use Flashbots Protect for transactions',
          'Adjust price range to reduce MEV attractiveness',
          'Consider splitting position across multiple pools'
        ]
      },
      {
        id: '2',
        type: 'optimization',
        title: 'Yield Optimization',
        description: 'You can increase yield by 2.3% by rebalancing to higher-yielding protocols with similar risk.',
        impact: 8.7,
        confidence: 73,
        timeframe: '1-2 days',
        actions: [
          'Move 30% of AAVE position to Compound',
          'Consider Curve stETH pool for better yields',
          'Optimize Uniswap V3 ranges for current volatility'
        ]
      },
      {
        id: '3',
        type: 'rebalancing',
        title: 'Diversification Improvement',
        description: 'Your portfolio is over-concentrated in Ethereum ecosystem. Consider multi-chain diversification.',
        impact: 12.4,
        confidence: 65,
        timeframe: '1 week',
        actions: [
          'Allocate 15% to Polygon DeFi protocols',
          'Consider Avalanche or BSC opportunities',
          'Maintain 70% Ethereum, 30% other chains'
        ]
      }
    ]);
  }, []);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatMessages]);

  const handleSendMessage = async () => {
    if (!currentMessage.trim() || isTyping) return;

    const userMessage: ChatMessage = {
      id: `user_${Date.now()}`,
      type: 'user',
      content: currentMessage,
      timestamp: Date.now()
    };

    setChatMessages(prev => [...prev, userMessage]);
    setCurrentMessage('');
    setIsTyping(true);

    // Simulate AI response
    setTimeout(() => {
      const responses = [
        'Based on your portfolio analysis, I can see that your MEV risk is elevated due to your large ETH/USDC position. This position represents 17% of your portfolio and is actively providing liquidity in a high-traffic price range.',
        'Your portfolio shows strong diversification across 8 protocols. The main risk factors I\'m monitoring are: 1) MEV exposure (82/100), 2) Liquidity concentration (65/100), and 3) Cross-chain bridge risk (58/100).',
        'I recommend implementing MEV protection for your largest positions. You could save approximately 0.8-1.2% annually by using Flashbots Protect or similar services.',
        'Your correlation analysis shows high correlation (0.92) between your ETH/USDC and stETH/ETH positions. Consider reducing this correlation by diversifying into different asset pairs.',
        'The stress test results indicate your portfolio would lose approximately 42% in a severe market crash scenario. Consider hedging strategies or reducing leverage to improve resilience.'
      ];

      const aiMessage: ChatMessage = {
        id: `ai_${Date.now()}`,
        type: 'ai',
        content: responses[Math.floor(Math.random() * responses.length)],
        timestamp: Date.now(),
        suggestions: [
          'Tell me more about this',
          'How do I implement this?',
          'What are the risks?',
          'Show me alternatives'
        ]
      };

      setChatMessages(prev => [...prev, aiMessage]);
      setIsTyping(false);
    }, 1500);
  };

  const handleSuggestionClick = (suggestion: string) => {
    setCurrentMessage(suggestion);
  };

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 80) return 'text-green-400';
    if (confidence >= 60) return 'text-yellow-400';
    return 'text-orange-400';
  };

  const getImpactColor = (impact: number) => {
    if (impact >= 10) return 'text-green-400';
    if (impact >= 5) return 'text-blue-400';
    return 'text-gray-400';
  };

  const getRiskColor = (score: number) => {
    if (score >= 80) return 'text-red-400';
    if (score >= 60) return 'text-orange-400';
    if (score >= 30) return 'text-yellow-400';
    return 'text-green-400';
  };

  if (userTier === 'basic') {
    return (
      <div className="space-y-6">
        <div className="text-center py-12">
          <div className="text-4xl mb-4">🤖</div>
          <h3 className="text-xl font-semibold text-white mb-2">AI-Powered Insights</h3>
          <p className="text-gray-400 mb-6">
            Get personalized risk explanations, optimization recommendations, and predictive analytics powered by advanced AI.
          </p>
          <button className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-medium">
            Upgrade to Professional →
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with Tab Navigation */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">AI Insights Hub</h2>
        
        <div className="flex space-x-2">
          {[
            { id: 'chat', label: 'AI Chat', icon: '💬' },
            { id: 'explanations', label: 'Risk Explanations', icon: '🔍' },
            { id: 'recommendations', label: 'Recommendations', icon: '💡' },
            { id: 'predictions', label: 'Predictions', icon: '🔮', premium: userTier !== 'institutional' && userTier !== 'enterprise' }
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              disabled={tab.premium}
              className={`flex items-center space-x-1 px-3 py-1 rounded text-sm transition-colors ${
                activeTab === tab.id
                  ? 'bg-blue-600 text-white'
                  : tab.premium
                  ? 'bg-gray-800 text-gray-500 cursor-not-allowed'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              <span>{tab.icon}</span>
              <span>{tab.label}</span>
              {tab.premium && <span className="text-xs">PRO</span>}
            </button>
          ))}
        </div>
      </div>

      {/* AI Chat Interface */}
      {activeTab === 'chat' && (
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg">
          <div className="flex flex-col h-96">
            {/* Chat Messages */}
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              {chatMessages.map((message) => (
                <div key={message.id} className={`flex ${message.type === 'user' ? 'justify-end' : 'justify-start'}`}>
                  <div className={`max-w-xs lg:max-w-md px-4 py-2 rounded-lg ${
                    message.type === 'user' 
                      ? 'bg-blue-600 text-white' 
                      : 'bg-gray-700 text-gray-100'
                  }`}>
                    <p className="text-sm">{message.content}</p>
                    <div className="text-xs opacity-70 mt-1">
                      {new Date(message.timestamp).toLocaleTimeString()}
                    </div>
                  </div>
                </div>
              ))}
              
              {isTyping && (
                <div className="flex justify-start">
                  <div className="bg-gray-700 text-gray-100 px-4 py-2 rounded-lg">
                    <div className="flex space-x-1">
                      <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce"></div>
                      <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '0.1s' }}></div>
                      <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '0.2s' }}></div>
                    </div>
                  </div>
                </div>
              )}
              
              <div ref={chatEndRef} />
            </div>

            {/* Quick Suggestions */}
            {chatMessages.length > 0 && chatMessages[chatMessages.length - 1].suggestions && (
              <div className="px-4 py-2 border-t border-gray-700">
                <div className="flex flex-wrap gap-2">
                  {chatMessages[chatMessages.length - 1].suggestions?.map((suggestion, index) => (
                    <button
                      key={index}
                      onClick={() => handleSuggestionClick(suggestion)}
                      className="text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 px-2 py-1 rounded transition-colors"
                    >
                      {suggestion}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Message Input */}
            <div className="p-4 border-t border-gray-700">
              <div className="flex space-x-2">
                <input
                  type="text"
                  value={currentMessage}
                  onChange={(e) => setCurrentMessage(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleSendMessage()}
                  placeholder="Ask about your portfolio risks..."
                  className="flex-1 bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500"
                  disabled={isTyping}
                />
                <button
                  onClick={handleSendMessage}
                  disabled={!currentMessage.trim() || isTyping}
                  className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white rounded-lg transition-colors"
                >
                  Send
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Risk Explanations */}
      {activeTab === 'explanations' && (
        <div className="space-y-4">
          {riskExplanations.map((explanation, index) => (
            <div key={index} className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-white">{explanation.factor}</h3>
                <span className={`text-lg font-bold ${getRiskColor(explanation.score)}`}>
                  {explanation.score}/100
                </span>
              </div>
              
              <div className="space-y-3">
                <div>
                  <div className="text-sm font-medium text-gray-300 mb-1">Explanation</div>
                  <div className="text-sm text-gray-400">{explanation.explanation}</div>
                </div>
                
                <div>
                  <div className="text-sm font-medium text-gray-300 mb-1">Potential Impact</div>
                  <div className="text-sm text-gray-400">{explanation.impact}</div>
                </div>
                
                <div>
                  <div className="text-sm font-medium text-gray-300 mb-1">AI Recommendation</div>
                  <div className="text-sm text-blue-400">{explanation.recommendation}</div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* AI Recommendations */}
      {activeTab === 'recommendations' && (
        <div className="space-y-4">
          {recommendations.map((rec) => (
            <div key={rec.id} className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
              <div className="flex items-start justify-between mb-4">
                <div>
                  <div className="flex items-center space-x-2 mb-2">
                    <h3 className="text-lg font-semibold text-white">{rec.title}</h3>
                    <span className={`text-xs px-2 py-1 rounded ${
                      rec.type === 'optimization' ? 'bg-green-600/20 text-green-400' :
                      rec.type === 'risk_reduction' ? 'bg-red-600/20 text-red-400' :
                      rec.type === 'opportunity' ? 'bg-blue-600/20 text-blue-400' :
                      'bg-yellow-600/20 text-yellow-400'
                    }`}>
                      {rec.type.replace('_', ' ').toUpperCase()}
                    </span>
                  </div>
                  <p className="text-sm text-gray-400">{rec.description}</p>
                </div>
                
                <div className="text-right">
                  <div className={`text-lg font-bold ${getImpactColor(rec.impact)}`}>
                    +{rec.impact.toFixed(1)}%
                  </div>
                  <div className="text-xs text-gray-400">Impact</div>
                </div>
              </div>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div>
                  <div className="text-sm text-gray-400">Confidence</div>
                  <div className={`text-sm font-medium ${getConfidenceColor(rec.confidence)}`}>
                    {rec.confidence}%
                  </div>
                </div>
                <div>
                  <div className="text-sm text-gray-400">Timeframe</div>
                  <div className="text-sm font-medium text-white">{rec.timeframe}</div>
                </div>
              </div>
              
              <div>
                <div className="text-sm font-medium text-gray-300 mb-2">Recommended Actions</div>
                <ul className="space-y-1">
                  {rec.actions.map((action, index) => (
                    <li key={index} className="text-sm text-gray-400 flex items-start">
                      <span className="text-blue-400 mr-2">•</span>
                      {action}
                    </li>
                  ))}
                </ul>
              </div>
              
              <div className="mt-4 flex space-x-2">
                <button className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm">
                  Implement
                </button>
                <button className="bg-gray-700 hover:bg-gray-600 text-gray-300 px-4 py-2 rounded text-sm">
                  Learn More
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Predictions (Premium Feature) */}
      {activeTab === 'predictions' && (
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
          <div className="text-center py-8">
            <div className="text-4xl mb-4">🔮</div>
            <h3 className="text-lg font-semibold text-white mb-2">AI Predictions</h3>
            <p className="text-gray-400 mb-4">
              Advanced predictive analytics and risk forecasting powered by machine learning models.
            </p>
            <button className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-lg">
              Upgrade to Institutional →
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default AIInsightsHub;
