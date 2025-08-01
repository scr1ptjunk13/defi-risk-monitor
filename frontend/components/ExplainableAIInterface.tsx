/**
 * ExplainableAIInterface Component
 * 
 * AI-powered risk analysis interface that provides:
 * - Intelligent risk explanations and recommendations
 * - Natural language risk summaries
 * - Predictive risk modeling
 * - Actionable insights and suggestions
 * - Interactive Q&A about portfolio risks
 */

import React, { useState, useEffect, useRef } from 'react';
import { RiskMetrics, RiskExplanation } from '../lib/api-client';
import { 
  BrainIcon, 
  MessageCircleIcon, 
  TrendingUpIcon, 
  AlertTriangleIcon, 
  InfoIcon, 
  SendIcon,
  LoadingSpinner,
  LightbulbIcon,
  ShieldIcon,
  TargetIcon
} from './Icons';
import { toast } from 'react-hot-toast';

interface ExplainableAIInterfaceProps {
  riskMetrics?: RiskMetrics;
  positionId?: string;
  userAddress?: string;
  className?: string;
}

interface AIInsight {
  id: string;
  type: 'explanation' | 'recommendation' | 'prediction' | 'warning';
  title: string;
  content: string;
  confidence: number;
  priority: 'low' | 'medium' | 'high' | 'critical';
  category: 'market' | 'protocol' | 'operational' | 'financial';
  timestamp: string;
  actionable: boolean;
  actions?: string[];
}

interface ChatMessage {
  id: string;
  type: 'user' | 'ai';
  content: string;
  timestamp: string;
  insights?: AIInsight[];
}

interface RiskPrediction {
  timeframe: '1h' | '24h' | '7d' | '30d';
  currentRisk: number;
  predictedRisk: number;
  confidence: number;
  factors: string[];
  scenario: 'optimistic' | 'realistic' | 'pessimistic';
}

const ExplainableAIInterface: React.FC<ExplainableAIInterfaceProps> = ({
  riskMetrics,
  positionId,
  userAddress,
  className = ''
}) => {
  const [activeTab, setActiveTab] = useState<'insights' | 'chat' | 'predictions' | 'recommendations'>('insights');
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [currentMessage, setCurrentMessage] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [aiInsights, setAiInsights] = useState<AIInsight[]>([]);
  const [riskPredictions, setRiskPredictions] = useState<RiskPrediction[]>([]);
  const chatEndRef = useRef<HTMLDivElement>(null);

  // Initialize AI insights
  useEffect(() => {
    const generateAIInsights = () => {
      const insights: AIInsight[] = [
        {
          id: '1',
          type: 'warning',
          title: 'Elevated MEV Risk Detected',
          content: 'Your ETH/USDC position is experiencing high MEV risk (72%) due to increased arbitrage opportunities. Large price movements in the past 6 hours have created profitable sandwich attack vectors.',
          confidence: 0.89,
          priority: 'high',
          category: 'operational',
          timestamp: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
          actionable: true,
          actions: [
            'Consider using MEV protection services like Flashbots Protect',
            'Split large transactions into smaller chunks',
            'Monitor slippage tolerance settings',
            'Use private mempools for sensitive transactions'
          ]
        },
        {
          id: '2',
          type: 'recommendation',
          title: 'Portfolio Diversification Opportunity',
          content: 'Analysis shows 78% correlation between your top 3 positions. Diversifying into uncorrelated assets could reduce overall portfolio risk by 15-20% while maintaining similar yield potential.',
          confidence: 0.76,
          priority: 'medium',
          category: 'financial',
          timestamp: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
          actionable: true,
          actions: [
            'Consider adding stable yield positions (Aave, Compound)',
            'Explore cross-chain opportunities on Polygon or Arbitrum',
            'Add defensive positions in major stablecoins',
            'Implement gradual rebalancing over 7-14 days'
          ]
        },
        {
          id: '3',
          type: 'prediction',
          title: 'Impermanent Loss Forecast',
          content: 'Based on current market volatility patterns and historical correlations, your liquidity positions have a 65% probability of experiencing 3-5% additional impermanent loss over the next 7 days.',
          confidence: 0.71,
          priority: 'medium',
          category: 'financial',
          timestamp: new Date(Date.now() - 4 * 60 * 60 * 1000).toISOString(),
          actionable: true,
          actions: [
            'Monitor ETH/USDC price ratio closely',
            'Consider partial position closure if divergence exceeds 15%',
            'Set automated rebalancing triggers',
            'Evaluate fee earnings vs IL impact daily'
          ]
        },
        {
          id: '4',
          type: 'explanation',
          title: 'Protocol Risk Assessment',
          content: 'Your protocol risk score (38%) is primarily driven by smart contract complexity rather than security concerns. Uniswap V3 has a strong security track record with 4 completed audits and no recent exploits.',
          confidence: 0.94,
          priority: 'low',
          category: 'protocol',
          timestamp: new Date(Date.now() - 6 * 60 * 60 * 1000).toISOString(),
          actionable: false
        }
      ];

      setAiInsights(insights);
    };

    const generateRiskPredictions = () => {
      const predictions: RiskPrediction[] = [
        {
          timeframe: '1h',
          currentRisk: riskMetrics ? parseFloat(riskMetrics.overall_risk_score) : 65,
          predictedRisk: 68,
          confidence: 0.85,
          factors: ['Market volatility increase', 'MEV activity spike'],
          scenario: 'realistic'
        },
        {
          timeframe: '24h',
          currentRisk: riskMetrics ? parseFloat(riskMetrics.overall_risk_score) : 65,
          predictedRisk: 62,
          confidence: 0.72,
          factors: ['Expected volatility normalization', 'Reduced trading volume'],
          scenario: 'optimistic'
        },
        {
          timeframe: '7d',
          currentRisk: riskMetrics ? parseFloat(riskMetrics.overall_risk_score) : 65,
          predictedRisk: 58,
          confidence: 0.68,
          factors: ['Market stabilization', 'Protocol upgrades', 'Improved liquidity'],
          scenario: 'realistic'
        },
        {
          timeframe: '30d',
          currentRisk: riskMetrics ? parseFloat(riskMetrics.overall_risk_score) : 65,
          predictedRisk: 72,
          confidence: 0.45,
          factors: ['Seasonal volatility patterns', 'Regulatory uncertainty', 'Market cycle analysis'],
          scenario: 'pessimistic'
        }
      ];

      setRiskPredictions(predictions);
    };

    generateAIInsights();
    generateRiskPredictions();
  }, [riskMetrics]);

  // Initialize chat with AI greeting
  useEffect(() => {
    const initialMessage: ChatMessage = {
      id: '1',
      type: 'ai',
      content: "Hello! I'm your AI risk analyst. I've analyzed your portfolio and identified several key insights. How can I help you understand and manage your DeFi risks today?",
      timestamp: new Date().toISOString()
    };
    setChatMessages([initialMessage]);
  }, []);

  // Auto-scroll chat to bottom
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatMessages]);

  const handleSendMessage = async () => {
    if (!currentMessage.trim()) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      type: 'user',
      content: currentMessage,
      timestamp: new Date().toISOString()
    };

    setChatMessages(prev => [...prev, userMessage]);
    setCurrentMessage('');
    setIsTyping(true);

    // Simulate AI response
    setTimeout(() => {
      const aiResponse = generateAIResponse(currentMessage);
      const aiMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        type: 'ai',
        content: aiResponse.content,
        timestamp: new Date().toISOString(),
        insights: aiResponse.insights
      };

      setChatMessages(prev => [...prev, aiMessage]);
      setIsTyping(false);
    }, 1500 + Math.random() * 1000);
  };

  const generateAIResponse = (userInput: string): { content: string; insights?: AIInsight[] } => {
    const input = userInput.toLowerCase();
    
    if (input.includes('mev') || input.includes('sandwich')) {
      return {
        content: "MEV (Maximal Extractable Value) risk is currently your highest concern at 72%. This is primarily due to your large ETH/USDC position being visible to arbitrageurs. The recent 15% price movement has created profitable sandwich attack opportunities.\n\nI recommend implementing MEV protection immediately. Would you like me to explain the specific protection strategies available?",
        insights: aiInsights.filter(i => i.category === 'operational')
      };
    }
    
    if (input.includes('impermanent loss') || input.includes('il')) {
      return {
        content: "Your current impermanent loss is 2.8%, which is relatively manageable. However, my models predict a 65% chance of additional 3-5% IL over the next week due to expected ETH volatility.\n\nThe good news is that your fee earnings (currently offsetting 65% of IL) are helping mitigate this risk. Should I analyze your fee earnings trajectory?",
        insights: aiInsights.filter(i => i.content.includes('impermanent'))
      };
    }
    
    if (input.includes('diversif') || input.includes('correlation')) {
      return {
        content: "Great question! Your portfolio shows 78% correlation between top positions, which concentrates risk. I've identified several diversification opportunities that could reduce risk by 15-20%:\n\n1. Add stable yield positions (Aave USDC: 8.2% APY)\n2. Cross-chain exposure (Arbitrum has 23% lower correlation)\n3. Defensive stablecoin positions\n\nWould you like me to create a specific rebalancing plan?",
        insights: aiInsights.filter(i => i.type === 'recommendation')
      };
    }
    
    if (input.includes('predict') || input.includes('future') || input.includes('forecast')) {
      return {
        content: "Based on my predictive models, I see mixed signals for your portfolio:\n\n• Short-term (24h): Risk likely to decrease to 62% as volatility normalizes\n• Medium-term (7d): Continued improvement to 58% with market stabilization\n• Long-term (30d): Potential increase to 72% due to seasonal patterns\n\nConfidence levels vary from 45-85%. Would you like me to explain the key factors driving these predictions?"
      };
    }
    
    // Default response
    return {
      content: "I understand you're asking about your portfolio risks. Let me analyze your specific situation:\n\nYour overall risk score of 65% is driven primarily by:\n• MEV exposure (72%) - your largest concern\n• Volatility risk (65%) - manageable but worth monitoring\n• Cross-chain exposure (55%) - moderate risk level\n\nWhat specific aspect would you like me to dive deeper into? I can explain any risk factor, provide predictions, or suggest optimization strategies."
    };
  };

  const getInsightIcon = (type: string) => {
    switch (type) {
      case 'warning': return <AlertTriangleIcon className="w-5 h-5 text-red-400" />;
      case 'recommendation': return <LightbulbIcon className="w-5 h-5 text-blue-400" />;
      case 'prediction': return <TrendingUpIcon className="w-5 h-5 text-purple-400" />;
      case 'explanation': return <InfoIcon className="w-5 h-5 text-green-400" />;
      default: return <BrainIcon className="w-5 h-5 text-gray-400" />;
    }
  };

  const getInsightColor = (priority: string) => {
    switch (priority) {
      case 'critical': return 'border-red-500/50 bg-red-900/10';
      case 'high': return 'border-orange-500/50 bg-orange-900/10';
      case 'medium': return 'border-yellow-500/50 bg-yellow-900/10';
      case 'low': return 'border-blue-500/50 bg-blue-900/10';
      default: return 'border-gray-500/50 bg-gray-900/10';
    }
  };

  const getPredictionColor = (current: number, predicted: number) => {
    const change = predicted - current;
    if (change > 5) return 'text-red-400';
    if (change < -5) return 'text-green-400';
    return 'text-yellow-400';
  };

  const renderInsights = () => (
    <div className="space-y-4">
      {aiInsights.map((insight) => (
        <div key={insight.id} className={`p-4 rounded-lg border ${getInsightColor(insight.priority)}`}>
          <div className="flex items-start gap-3">
            {getInsightIcon(insight.type)}
            <div className="flex-1">
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-medium text-white">{insight.title}</h4>
                <div className="flex items-center gap-2">
                  <span className={`px-2 py-1 text-xs rounded ${
                    insight.priority === 'critical' ? 'bg-red-900/50 text-red-400' :
                    insight.priority === 'high' ? 'bg-orange-900/50 text-orange-400' :
                    insight.priority === 'medium' ? 'bg-yellow-900/50 text-yellow-400' :
                    'bg-blue-900/50 text-blue-400'
                  }`}>
                    {insight.priority.toUpperCase()}
                  </span>
                  <span className="text-xs text-gray-400">
                    {Math.round(insight.confidence * 100)}% confidence
                  </span>
                </div>
              </div>
              
              <p className="text-sm text-gray-300 mb-3">{insight.content}</p>
              
              {insight.actionable && insight.actions && (
                <div>
                  <h5 className="text-sm font-medium text-gray-300 mb-2">Recommended Actions:</h5>
                  <ul className="space-y-1">
                    {insight.actions.map((action, index) => (
                      <li key={index} className="text-xs text-gray-400 flex items-start gap-2">
                        <TargetIcon className="w-3 h-3 text-blue-400 mt-0.5 flex-shrink-0" />
                        {action}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              
              <div className="flex items-center justify-between mt-3 pt-3 border-t border-gray-600">
                <span className="text-xs text-gray-500">
                  {new Date(insight.timestamp).toLocaleString()}
                </span>
                <span className="text-xs text-gray-500 capitalize">
                  {insight.category} • {insight.type}
                </span>
              </div>
            </div>
          </div>
        </div>
      ))}
    </div>
  );

  const renderChat = () => (
    <div className="flex flex-col h-96">
      <div className="flex-1 overflow-y-auto space-y-4 p-4 bg-gray-900/30 rounded-lg">
        {chatMessages.map((message) => (
          <div key={message.id} className={`flex ${message.type === 'user' ? 'justify-end' : 'justify-start'}`}>
            <div className={`max-w-xs lg:max-w-md px-4 py-2 rounded-lg ${
              message.type === 'user' 
                ? 'bg-blue-600 text-white' 
                : 'bg-gray-700 text-gray-100'
            }`}>
              <p className="text-sm">{message.content}</p>
              {message.insights && message.insights.length > 0 && (
                <div className="mt-2 pt-2 border-t border-gray-600">
                  <p className="text-xs text-gray-400 mb-1">Related insights:</p>
                  {message.insights.map((insight) => (
                    <div key={insight.id} className="text-xs text-blue-300 hover:text-blue-200 cursor-pointer">
                      • {insight.title}
                    </div>
                  ))}
                </div>
              )}
              <div className="text-xs text-gray-400 mt-1">
                {new Date(message.timestamp).toLocaleTimeString()}
              </div>
            </div>
          </div>
        ))}
        
        {isTyping && (
          <div className="flex justify-start">
            <div className="bg-gray-700 text-gray-100 px-4 py-2 rounded-lg">
              <div className="flex items-center gap-2">
                <LoadingSpinner className="w-4 h-4" />
                <span className="text-sm">AI is thinking...</span>
              </div>
            </div>
          </div>
        )}
        
        <div ref={chatEndRef} />
      </div>
      
      <div className="flex gap-2 mt-4">
        <input
          type="text"
          value={currentMessage}
          onChange={(e) => setCurrentMessage(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && handleSendMessage()}
          placeholder="Ask about your portfolio risks..."
          className="flex-1 bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm"
          disabled={isTyping}
        />
        <button
          onClick={handleSendMessage}
          disabled={!currentMessage.trim() || isTyping}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white rounded-lg transition-colors"
        >
          <SendIcon className="w-4 h-4" />
        </button>
      </div>
    </div>
  );

  const renderPredictions = () => (
    <div className="space-y-4">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {riskPredictions.map((prediction) => (
          <div key={prediction.timeframe} className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
            <div className="flex items-center justify-between mb-3">
              <h4 className="font-medium text-white">{prediction.timeframe} Forecast</h4>
              <span className="text-xs text-gray-400">
                {Math.round(prediction.confidence * 100)}% confidence
              </span>
            </div>
            
            <div className="flex items-center justify-between mb-3">
              <div>
                <div className="text-sm text-gray-400">Current Risk</div>
                <div className="text-lg font-semibold text-white">
                  {prediction.currentRisk.toFixed(1)}%
                </div>
              </div>
              
              <div className="text-center">
                <div className="text-2xl">→</div>
              </div>
              
              <div>
                <div className="text-sm text-gray-400">Predicted Risk</div>
                <div className={`text-lg font-semibold ${getPredictionColor(prediction.currentRisk, prediction.predictedRisk)}`}>
                  {prediction.predictedRisk.toFixed(1)}%
                </div>
              </div>
            </div>
            
            <div className="mb-3">
              <div className="text-sm text-gray-400 mb-1">Key Factors:</div>
              <ul className="space-y-1">
                {prediction.factors.map((factor, index) => (
                  <li key={index} className="text-xs text-gray-300 flex items-start gap-2">
                    <span className="w-1 h-1 bg-gray-500 rounded-full mt-2 flex-shrink-0" />
                    {factor}
                  </li>
                ))}
              </ul>
            </div>
            
            <div className={`px-2 py-1 text-xs rounded text-center ${
              prediction.scenario === 'optimistic' ? 'bg-green-900/50 text-green-400' :
              prediction.scenario === 'pessimistic' ? 'bg-red-900/50 text-red-400' :
              'bg-yellow-900/50 text-yellow-400'
            }`}>
              {prediction.scenario.toUpperCase()} SCENARIO
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderRecommendations = () => (
    <div className="space-y-4">
      {aiInsights.filter(i => i.type === 'recommendation' || i.actionable).map((insight) => (
        <div key={insight.id} className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <div className="flex items-start gap-3">
            <ShieldIcon className="w-5 h-5 text-blue-400 mt-1" />
            <div className="flex-1">
              <h4 className="font-medium text-white mb-2">{insight.title}</h4>
              <p className="text-sm text-gray-300 mb-3">{insight.content}</p>
              
              {insight.actions && (
                <div className="space-y-2">
                  {insight.actions.map((action, index) => (
                    <div key={index} className="flex items-center gap-3 p-2 bg-gray-800/50 rounded">
                      <input type="checkbox" className="rounded border-gray-600 bg-gray-700 text-blue-600" />
                      <span className="text-sm text-gray-300">{action}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  );

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <BrainIcon className="w-6 h-6 text-blue-400" />
          <div>
            <h3 className="text-lg font-semibold text-white">AI Risk Analyst</h3>
            <p className="text-sm text-gray-400">
              Intelligent risk analysis and recommendations
            </p>
          </div>
        </div>
        
        <div className="flex items-center gap-2 text-xs text-gray-400">
          <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
          AI Online
        </div>
      </div>

      {/* Tab Navigation */}
      <div className="flex border-b border-gray-700 mb-6">
        {[
          { key: 'insights', label: 'AI Insights', count: aiInsights.length },
          { key: 'chat', label: 'Chat', count: null },
          { key: 'predictions', label: 'Predictions', count: riskPredictions.length },
          { key: 'recommendations', label: 'Actions', count: aiInsights.filter(i => i.actionable).length }
        ].map(({ key, label, count }) => (
          <button
            key={key}
            onClick={() => setActiveTab(key as any)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === key
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-gray-300'
            }`}
          >
            {label}
            {count !== null && (
              <span className="ml-2 px-2 py-1 text-xs bg-gray-700 rounded-full">
                {count}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="min-h-[400px]">
        {activeTab === 'insights' && renderInsights()}
        {activeTab === 'chat' && renderChat()}
        {activeTab === 'predictions' && renderPredictions()}
        {activeTab === 'recommendations' && renderRecommendations()}
      </div>
    </div>
  );
};

export default ExplainableAIInterface;
