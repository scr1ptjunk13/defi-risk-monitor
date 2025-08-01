import React, { useState, useEffect } from 'react';
import {
  Card,
  Table,
  Tag,
  Button,
  Space,
  Statistic,
  Row,
  Col,
  Modal,
  Typography,
  Collapse,
  Badge,
  Tooltip,
  Select,
  DatePicker,
  Empty
} from 'antd';
import {
  BugOutlined,
  DeleteOutlined,
  EyeOutlined,
  DownloadOutlined,
  ReloadOutlined,
  WarningOutlined,
  InfoCircleOutlined
} from '@ant-design/icons';
import { errorHandler, EnhancedError, ErrorSeverity, ErrorCategory } from '../lib/error-handling';

const { Text, Paragraph } = Typography;
const { Panel } = Collapse;
const { RangePicker } = DatePicker;

interface ErrorDashboardProps {
  visible?: boolean;
  onClose?: () => void;
}

export const ErrorDashboard: React.FC<ErrorDashboardProps> = ({ visible = false, onClose }) => {
  const [errors, setErrors] = useState<EnhancedError[]>([]);
  const [filteredErrors, setFilteredErrors] = useState<EnhancedError[]>([]);
  const [selectedError, setSelectedError] = useState<EnhancedError | null>(null);
  const [detailsVisible, setDetailsVisible] = useState(false);
  const [filters, setFilters] = useState({
    severity: undefined as ErrorSeverity | undefined,
    category: undefined as ErrorCategory | undefined,
    dateRange: undefined as [any, any] | undefined
  });

  useEffect(() => {
    refreshErrors();
    const interval = setInterval(refreshErrors, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    applyFilters();
  }, [errors, filters]);

  const refreshErrors = () => {
    const recentErrors = errorHandler.getRecentErrors(100);
    setErrors(recentErrors);
  };

  const applyFilters = () => {
    let filtered = [...errors];

    if (filters.severity) {
      filtered = filtered.filter(error => error.severity === filters.severity);
    }

    if (filters.category) {
      filtered = filtered.filter(error => error.category === filters.category);
    }

    if (filters.dateRange) {
      const [start, end] = filters.dateRange;
      filtered = filtered.filter(error => {
        const errorDate = new Date(error.context.timestamp);
        return errorDate >= start.toDate() && errorDate <= end.toDate();
      });
    }

    setFilteredErrors(filtered);
  };

  const getSeverityColor = (severity: ErrorSeverity) => {
    const colors = {
      [ErrorSeverity.LOW]: 'blue',
      [ErrorSeverity.MEDIUM]: 'orange',
      [ErrorSeverity.HIGH]: 'red',
      [ErrorSeverity.CRITICAL]: 'purple'
    };
    return colors[severity];
  };

  const getCategoryIcon = (category: ErrorCategory) => {
    const icons = {
      [ErrorCategory.NETWORK]: '🌐',
      [ErrorCategory.VALIDATION]: '✅',
      [ErrorCategory.AUTHENTICATION]: '🔐',
      [ErrorCategory.AUTHORIZATION]: '🚫',
      [ErrorCategory.BLOCKCHAIN]: '⛓️',
      [ErrorCategory.API]: '🔌',
      [ErrorCategory.UI]: '🖥️',
      [ErrorCategory.UNKNOWN]: '❓'
    };
    return icons[category];
  };

  const showErrorDetails = (error: EnhancedError) => {
    setSelectedError(error);
    setDetailsVisible(true);
  };

  const downloadErrorReport = (error: EnhancedError) => {
    const report = {
      ...error,
      exportedAt: new Date().toISOString(),
      userAgent: navigator.userAgent,
      viewport: {
        width: window.innerWidth,
        height: window.innerHeight
      }
    };

    const blob = new Blob([JSON.stringify(report, null, 2)], {
      type: 'application/json'
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `error-${error.id}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const downloadAllErrors = () => {
    const report = {
      errors: filteredErrors,
      stats: errorHandler.getErrorStats(),
      exportedAt: new Date().toISOString(),
      filters: filters
    };

    const blob = new Blob([JSON.stringify(report, null, 2)], {
      type: 'application/json'
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `error-report-${new Date().toISOString().split('T')[0]}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const clearAllErrors = () => {
    Modal.confirm({
      title: 'Clear All Errors',
      content: 'Are you sure you want to clear all error logs? This action cannot be undone.',
      okText: 'Clear',
      okType: 'danger',
      onOk: () => {
        errorHandler.clearErrorLog();
        refreshErrors();
      }
    });
  };

  const stats = errorHandler.getErrorStats();

  const columns = [
    {
      title: 'Time',
      dataIndex: ['context', 'timestamp'],
      key: 'timestamp',
      width: 120,
      render: (timestamp: Date) => new Date(timestamp).toLocaleTimeString(),
      sorter: (a: EnhancedError, b: EnhancedError) => 
        new Date(b.context.timestamp).getTime() - new Date(a.context.timestamp).getTime()
    },
    {
      title: 'Severity',
      dataIndex: 'severity',
      key: 'severity',
      width: 100,
      render: (severity: ErrorSeverity) => (
        <Tag color={getSeverityColor(severity)}>
          {severity.toUpperCase()}
        </Tag>
      ),
      filters: Object.values(ErrorSeverity).map(severity => ({
        text: severity.toUpperCase(),
        value: severity
      })),
      onFilter: (value: any, record: EnhancedError) => record.severity === value
    },
    {
      title: 'Category',
      dataIndex: 'category',
      key: 'category',
      width: 120,
      render: (category: ErrorCategory) => (
        <Space>
          <span>{getCategoryIcon(category)}</span>
          <Text>{category}</Text>
        </Space>
      ),
      filters: Object.values(ErrorCategory).map(category => ({
        text: category,
        value: category
      })),
      onFilter: (value: any, record: EnhancedError) => record.category === value
    },
    {
      title: 'Component',
      dataIndex: ['context', 'component'],
      key: 'component',
      width: 120,
      render: (component: string) => <Tag>{component}</Tag>
    },
    {
      title: 'Message',
      dataIndex: 'message',
      key: 'message',
      ellipsis: true,
      render: (message: string) => (
        <Tooltip title={message}>
          <Text>{message}</Text>
        </Tooltip>
      )
    },
    {
      title: 'Status',
      key: 'status',
      width: 100,
      render: (record: EnhancedError) => (
        <Space direction="vertical" size="small">
          {record.retryable && <Tag color="blue">Retryable</Tag>}
          {record.recoverable && <Tag color="green">Recoverable</Tag>}
        </Space>
      )
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 120,
      render: (record: EnhancedError) => (
        <Space>
          <Button
            size="small"
            icon={<EyeOutlined />}
            onClick={() => showErrorDetails(record)}
          />
          <Button
            size="small"
            icon={<DownloadOutlined />}
            onClick={() => downloadErrorReport(record)}
          />
        </Space>
      )
    }
  ];

  if (!visible) return null;

  return (
    <Modal
      title={
        <Space>
          <BugOutlined />
          Error Dashboard
          <Badge count={errors.length} showZero />
        </Space>
      }
      open={visible}
      onCancel={onClose}
      width={1200}
      footer={null}
      bodyStyle={{ padding: '16px' }}
    >
      {/* Statistics */}
      <Row gutter={16} style={{ marginBottom: '16px' }}>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title="Total Errors"
              value={stats.total}
              prefix={<BugOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title="Critical"
              value={stats.bySeverity[ErrorSeverity.CRITICAL]}
              valueStyle={{ color: '#722ed1' }}
              prefix={<WarningOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title="High"
              value={stats.bySeverity[ErrorSeverity.HIGH]}
              valueStyle={{ color: '#f5222d' }}
              prefix={<WarningOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title="Medium/Low"
              value={stats.bySeverity[ErrorSeverity.MEDIUM] + stats.bySeverity[ErrorSeverity.LOW]}
              valueStyle={{ color: '#fa8c16' }}
              prefix={<InfoCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* Filters */}
      <Card size="small" style={{ marginBottom: '16px' }}>
        <Space wrap>
          <Select
            placeholder="Filter by severity"
            style={{ width: 150 }}
            allowClear
            value={filters.severity}
            onChange={(value) => setFilters({ ...filters, severity: value })}
          >
            {Object.values(ErrorSeverity).map(severity => (
              <Select.Option key={severity} value={severity}>
                {severity.toUpperCase()}
              </Select.Option>
            ))}
          </Select>

          <Select
            placeholder="Filter by category"
            style={{ width: 150 }}
            allowClear
            value={filters.category}
            onChange={(value) => setFilters({ ...filters, category: value })}
          >
            {Object.values(ErrorCategory).map(category => (
              <Select.Option key={category} value={category}>
                {getCategoryIcon(category)} {category}
              </Select.Option>
            ))}
          </Select>

          <RangePicker
            placeholder={['Start date', 'End date']}
            value={filters.dateRange}
            onChange={(dates) => setFilters({ ...filters, dateRange: dates as any })}
          />

          <Button icon={<ReloadOutlined />} onClick={refreshErrors}>
            Refresh
          </Button>

          <Button icon={<DownloadOutlined />} onClick={downloadAllErrors}>
            Export All
          </Button>

          <Button 
            icon={<DeleteOutlined />} 
            danger 
            onClick={clearAllErrors}
            disabled={errors.length === 0}
          >
            Clear All
          </Button>
        </Space>
      </Card>

      {/* Error Table */}
      <Table
        columns={columns}
        dataSource={filteredErrors}
        rowKey="id"
        size="small"
        pagination={{
          pageSize: 10,
          showSizeChanger: true,
          showQuickJumper: true,
          showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} errors`
        }}
        locale={{
          emptyText: (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="No errors found"
            />
          )
        }}
      />

      {/* Error Details Modal */}
      <Modal
        title={
          <Space>
            <BugOutlined />
            Error Details
            {selectedError && <Tag color={getSeverityColor(selectedError.severity)}>
              {selectedError.severity.toUpperCase()}
            </Tag>}
          </Space>
        }
        open={detailsVisible}
        onCancel={() => setDetailsVisible(false)}
        width={800}
        footer={[
          <Button key="download" icon={<DownloadOutlined />} onClick={() => selectedError && downloadErrorReport(selectedError)}>
            Download Report
          </Button>,
          <Button key="close" onClick={() => setDetailsVisible(false)}>
            Close
          </Button>
        ]}
      >
        {selectedError && (
          <Space direction="vertical" style={{ width: '100%' }}>
            <Card size="small" title="Error Information">
              <Row gutter={16}>
                <Col span={12}>
                  <Text strong>ID:</Text> <Text code>{selectedError.id}</Text>
                </Col>
                <Col span={12}>
                  <Text strong>Timestamp:</Text> <Text>{new Date(selectedError.context.timestamp).toLocaleString()}</Text>
                </Col>
                <Col span={12}>
                  <Text strong>Component:</Text> <Text>{selectedError.context.component}</Text>
                </Col>
                <Col span={12}>
                  <Text strong>Action:</Text> <Text>{selectedError.context.action}</Text>
                </Col>
                <Col span={12}>
                  <Text strong>Retryable:</Text> <Tag color={selectedError.retryable ? 'green' : 'red'}>{selectedError.retryable ? 'Yes' : 'No'}</Tag>
                </Col>
                <Col span={12}>
                  <Text strong>Recoverable:</Text> <Tag color={selectedError.recoverable ? 'green' : 'red'}>{selectedError.recoverable ? 'Yes' : 'No'}</Tag>
                </Col>
              </Row>
            </Card>

            <Card size="small" title="Messages">
              <Space direction="vertical" style={{ width: '100%' }}>
                <div>
                  <Text strong>Technical Message:</Text>
                  <Paragraph code copyable style={{ marginTop: '4px' }}>
                    {selectedError.message}
                  </Paragraph>
                </div>
                <div>
                  <Text strong>User Message:</Text>
                  <Paragraph style={{ marginTop: '4px' }}>
                    {selectedError.userMessage}
                  </Paragraph>
                </div>
              </Space>
            </Card>

            {selectedError.suggestions && selectedError.suggestions.length > 0 && (
              <Card size="small" title="Suggestions">
                <ul>
                  {selectedError.suggestions.map((suggestion, index) => (
                    <li key={index}>{suggestion}</li>
                  ))}
                </ul>
              </Card>
            )}

            <Collapse ghost>
              <Panel header="Technical Details" key="technical">
                <Space direction="vertical" style={{ width: '100%' }}>
                  {selectedError.stack && (
                    <div>
                      <Text strong>Stack Trace:</Text>
                      <Paragraph
                        code
                        copyable
                        style={{
                          marginTop: '4px',
                          maxHeight: '200px',
                          overflow: 'auto',
                          fontSize: '11px'
                        }}
                      >
                        {selectedError.stack}
                      </Paragraph>
                    </div>
                  )}

                  {selectedError.context.additionalData && (
                    <div>
                      <Text strong>Additional Data:</Text>
                      <Paragraph
                        code
                        copyable
                        style={{
                          marginTop: '4px',
                          maxHeight: '200px',
                          overflow: 'auto',
                          fontSize: '11px'
                        }}
                      >
                        {JSON.stringify(selectedError.context.additionalData, null, 2)}
                      </Paragraph>
                    </div>
                  )}

                  <div>
                    <Text strong>Context:</Text>
                    <Paragraph
                      code
                      copyable
                      style={{
                        marginTop: '4px',
                        maxHeight: '200px',
                        overflow: 'auto',
                        fontSize: '11px'
                      }}
                    >
                      {JSON.stringify({
                        url: selectedError.context.url,
                        userAgent: selectedError.context.userAgent,
                        timestamp: selectedError.context.timestamp
                      }, null, 2)}
                    </Paragraph>
                  </div>
                </Space>
              </Panel>
            </Collapse>
          </Space>
        )}
      </Modal>
    </Modal>
  );
};
