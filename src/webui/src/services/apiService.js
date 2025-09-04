// services/apiService.js

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8080/api';

class ApiService {
  constructor() {
    this.baseURL = API_BASE_URL;
  }

  async request(endpoint, options = {}) {
    const url = `${this.baseURL}${endpoint}`;
    const config = {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    };

    try {
      const response = await fetch(url, config);
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      return await response.json();
    } catch (error) {
      console.error('API request failed:', error);
      throw error;
    }
  }

  // Dashboard endpoints
  async getDashboardStats() {
    return this.request('/dashboard/stats');
  }

  async getNetworkActivity(timeRange = '24h') {
    return this.request(`/network/activity?range=${timeRange}`);
  }

  async getTopThreats(limit = 10) {
    return this.request(`/threats/top?limit=${limit}`);
  }

  async getSystemMetrics() {
    return this.request('/system/metrics');
  }

  // Session management endpoints
  async getActiveSessions() {
    return this.request('/sessions/active');
  }

  async getSessionById(sessionId) {
    return this.request(`/sessions/${sessionId}`);
  }

  async terminateSession(sessionId) {
    return this.request(`/sessions/${sessionId}/terminate`, {
      method: 'POST',
    });
  }

  // Service management endpoints
  async getServices() {
    return this.request('/services');
  }

  async getServiceStatus(serviceName) {
    return this.request(`/services/${serviceName}/status`);
  }

  async restartService(serviceName) {
    return this.request(`/services/${serviceName}/restart`, {
      method: 'POST',
    });
  }

  // Container management endpoints
  async getContainers() {
    return this.request('/containers');
  }

  async getContainerLogs(containerId, lines = 100) {
    return this.request(`/containers/${containerId}/logs?lines=${lines}`);
  }

  async startContainer(containerId) {
    return this.request(`/containers/${containerId}/start`, {
      method: 'POST',
    });
  }

  async stopContainer(containerId) {
    return this.request(`/containers/${containerId}/stop`, {
      method: 'POST',
    });
  }

  // Data capture endpoints
  async getCaptureStats() {
    return this.request('/capture/stats');
  }

  async startCapture(config) {
    return this.request('/capture/start', {
      method: 'POST',
      body: JSON.stringify(config),
    });
  }

  async stopCapture(captureId) {
    return this.request(`/capture/${captureId}/stop`, {
      method: 'POST',
    });
  }

  async getCaptures() {
    return this.request('/capture/list');
  }

  async downloadCapture(captureId) {
    const response = await fetch(`${this.baseURL}/capture/${captureId}/download`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.blob();
  }

  // Configuration endpoints
  async getConfiguration() {
    return this.request('/config');
  }

  async updateConfiguration(config) {
    return this.request('/config', {
      method: 'PUT',
      body: JSON.stringify(config),
    });
  }

  async getServiceConfigs() {
    return this.request('/config/services');
  }

  async updateServiceConfig(serviceName, config) {
    return this.request(`/config/services/${serviceName}`, {
      method: 'PUT',
      body: JSON.stringify(config),
    });
  }

  // Filter management endpoints
  async getIpFilters() {
    return this.request('/filters/ip');
  }

  async updateIpFilters(filters) {
    return this.request('/filters/ip', {
      method: 'PUT',
      body: JSON.stringify(filters),
    });
  }

  async getPortFilters() {
    return this.request('/filters/port');
  }

  async updatePortFilters(filters) {
    return this.request('/filters/port', {
      method: 'PUT',
      body: JSON.stringify(filters),
    });
  }

  // Export endpoints
  async exportData(format = 'json', timeRange = '24h') {
    const response = await fetch(
      `${this.baseURL}/export?format=${format}&range=${timeRange}`
    );
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    
    return response.blob();
  }

  async exportLogs(serviceNames = [], timeRange = '24h') {
    const services = Array.isArray(serviceNames) ? serviceNames.join(',') : serviceNames;
    const response = await fetch(
      `${this.baseURL}/export/logs?services=${services}&range=${timeRange}`
    );
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    
    return response.blob();
  }

  // Real-time data (WebSocket simulation with polling)
  startPolling(callback, interval = 30000) {
    const poll = async () => {
      try {
        const [stats, activity, threats] = await Promise.all([
          this.getDashboardStats(),
          this.getNetworkActivity(),
          this.getTopThreats(),
        ]);
        
        callback({
          stats,
          activity,
          threats,
          timestamp: new Date().toISOString(),
        });
      } catch (error) {
        console.error('Polling error:', error);
      }
    };

    // Initial call
    poll();
    
    // Set up interval
    const intervalId = setInterval(poll, interval);
    
    // Return cleanup function
    return () => clearInterval(intervalId);
  }
}

// Create singleton instance
const apiService = new ApiService();

export default apiService;
