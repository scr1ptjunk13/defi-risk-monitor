import React, { useEffect, useState } from 'react';
import { Card, Statistic, Row, Col, Table, Spin, Alert } from 'antd';
import { Pie, Line } from '@ant-design/charts';
import { apiClient } from '../lib/api-client';

interface PositionSummary {
  id: string;
  pool_address: string;
  current_value_usd: string;
  entry_value_usd: string;
  pnl_usd: string;
  fees_usd: string;
  risk_score?: string;
  protocol: string;
  chain: string;
}

interface PortfolioSummary {
  user_address: string;
  total_value_usd: string;
  total_pnl_usd: string;
  total_fees_usd: string;
  positions: PositionSummary[];
  protocol_breakdown: Record<string, string>;
  chain_breakdown: Record<string, string>;
  risk_aggregation: Record<string, string>;
  historical_values: Array<[string, string]>;
}

const PortfolioDashboard: React.FC<{ userAddress: string }> = ({ userAddress }) => {
  const [portfolio, setPortfolio] = useState<PortfolioSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    apiClient.getPortfolio(userAddress)
      .then(setPortfolio)
      .catch((err: any) => setError(err?.message || 'Error loading portfolio'))
      .finally(() => setLoading(false));
  }, [userAddress]);

  if (loading) return <Spin tip="Loading portfolio..." />;
  if (error) return <Alert type="error" message={error} />;
  if (!portfolio) return <Alert type="warning" message="No portfolio data found." />;

  const protocolData = Object.entries(portfolio.protocol_breakdown).map(([type, value]) => ({ type, value: Number(value) }));
  const chainData = Object.entries(portfolio.chain_breakdown).map(([type, value]) => ({ type, value: Number(value) }));
  const historicalData = portfolio.historical_values.map(([date, value]) => ({ date, value: Number(value) }));

  const columns = [
    { title: 'Pool', dataIndex: 'pool_address', key: 'pool_address' },
    { title: 'Current Value (USD)', dataIndex: 'current_value_usd', key: 'current_value_usd' },
    { title: 'PnL (USD)', dataIndex: 'pnl_usd', key: 'pnl_usd' },
    { title: 'Fees (USD)', dataIndex: 'fees_usd', key: 'fees_usd' },
    { title: 'Risk Score', dataIndex: 'risk_score', key: 'risk_score' },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol' },
    { title: 'Chain', dataIndex: 'chain', key: 'chain' }
  ];

  return (
    <div style={{ padding: 24 }}>
      <Row gutter={16}>
        <Col span={8}>
          <Card>
            <Statistic title="Total Portfolio Value" value={portfolio.total_value_usd} prefix="$" precision={2} />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic title="Total PnL" value={portfolio.total_pnl_usd} prefix="$" precision={2} />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic title="Total Fees" value={portfolio.total_fees_usd} prefix="$" precision={2} />
          </Card>
        </Col>
      </Row>
      <Row gutter={16} style={{ marginTop: 24 }}>
        <Col span={12}>
          <Card title="Protocol Diversification">
            <Pie data={protocolData} angleField="value" colorField="type" />
          </Card>
        </Col>
        <Col span={12}>
          <Card title="Chain Diversification">
            <Pie data={chainData} angleField="value" colorField="type" />
          </Card>
        </Col>
      </Row>
      <Row gutter={16} style={{ marginTop: 24 }}>
        <Col span={24}>
          <Card title="Portfolio Value Over Time">
            <Line data={historicalData} xField="date" yField="value" />
          </Card>
        </Col>
      </Row>
      <Row gutter={16} style={{ marginTop: 24 }}>
        <Col span={24}>
          <Card title="All Positions">
            <Table columns={columns} dataSource={portfolio.positions} rowKey="id" pagination={false} />
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default PortfolioDashboard;
