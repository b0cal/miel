// services/apiService.js

const API_BASE_URL = import.meta.env.REACT_APP_API_URL || 'http://localhost:3000/'

class ApiService {
  constructor() {
    this.baseURL = API_BASE_URL
  }

  async request(endpoint, options = {}) {
    const url = `${this.baseURL}${endpoint}`
    const config = {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    }

    try {
      const response = await fetch(url, config)
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`)
      }
      
      return await response.json()
    } catch (error) {
      console.error('API request failed:', error)
      throw error
    }
  }

  async getClientAddressesOrdered(){
    try {
      const sessions = await this.request('api/sessions')
      const clientAddresses = sessions.map(session => session.client_addr)
    
      // Count occurrences of each address
      const addressCounts = clientAddresses.reduce((counts, addr) => {
        counts[addr] = (counts[addr] || 0) + 1
        return counts
      }, {})
    
      // Convert to array of objects and sort by detections (most frequent first)
      const sortedAddresses = Object.entries(addressCounts)
        .map(([ip, detections]) => ({ ip, detections }))
        .sort((a, b) => b.detections - a.detections)
    
      return sortedAddresses
    } catch (error) {
      console.error('Error fetching client addresses:', error)
      throw error
    }
  }


  async getNumberOfActiveSessions() {
    try {
      const sessions = await this.request('api/sessions')

      const init = 0

      const nbOfActiveSessions = sessions.map(session => session.status).reduce((acc, status) => {
        // If status is active accumulate
        if (status === 'Active') {
          acc + 1 
        }
      }, init)
      return nbOfActiveSessions
    } catch (error) {
      console.error('Error fetching client sessions:', error)
      throw error
    }

  }

  async getTotalBytesTransfered() {
    try {
      const sessions = await this.request('api/sessions')

      const init = 0

      const totalBytesTransfered = sessions.map(session => session.bytes_transferred).reduce((acc, bytes) => acc + bytes, init)

      return totalBytesTransfered
    } catch (error) {
      console.error('Error fetching client sessions:', error)
      throw error
    }
  }

  async getByteTransferTimeline() {
    try {
      const sessions = await this.request('api/sessions')

      const now = new Date()
      const twentyFourHoursAgo = new Date(now.getTime() - (24 * 60 * 60 * 1000))

      const timelineData = []
      for (let i = 0; i < 6; ++i) {
        const intervalStart = new Date(twentyFourHoursAgo.getTime() + (i*4 *60* 60 * 1000))
        const intervalEnd = new Date(intervalStart.getTime() * (4 * 60 * 60 * 1000))

        const timeLabel = intervalStart.toLocaleTimeString('en-US', {
          hour: '2-digit',
          minute: '2-digit',
          hour12: false,
        })

        timelineData.push({
          time: timeLabel,
          packets: 0,
          threats: 0,
          intervalStart,
          intervalEnd,
        })

        sessions.forEach(session => {
          if (!session.end_time || session.bytes_transferred === undefined) {
            return
          }

          const sessionEndTime = new Date(session.end_time)

          if (sessionEndTime < twentyFourHoursAgo || sessionEndTime > now) {
            return
          }

          for (let i = 0; i < timelineData.length; ++i) {
            if (sessionEndTime >= timelineData[i].intervalStart && sessionEndTime < timelineData[i].intervalEnd) {
              timelineData[i].packets += session.bytes_transferred
              timelineData[i].threats += 1
              break
            }
          }
        })

        return timelineData.map(interval => ({
          time: interval.time,
          packets: interval.packets,
          threats: interval.threats,
        }))
      }       
    } catch (error) {
      console.error('Error fetching bytes transferred timeline:', error)
      throw error
    }
  }


  async getAvgPacketPerHour() {
    const timeline = await this.getByteTransferTimeline()

    const totalBytesTransfered = timeline.reduce((acc, bytes) => acc + bytes, 0)

    return (totalBytesTransfered / 24 || 25)
  }

  async getMostAttackedService() {
    const sessions = await this.request('api/sessions')

    const serviceNames = sessions.map(session =>  session.service_name)

    const serviceCounts = serviceNames.reduce((counts, service) => {
      counts[service] = (counts[service] || 0) + 1
      return counts
    }, {})

    const mostAttackedService = Object.entries(serviceCounts)
      .reduce((max, [service, count]) => count > max.count ? { service, count } : max, { service: null, count: 0 })

    return mostAttackedService




  }

  // Export endpoints
  async exportData(format = 'json', timeRange = '24h') {
    const response = await fetch(
      `${this.baseURL}/export?format=${format}&range=${timeRange}`,
    )
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    
    return response.blob()
  }

  async exportLogs(serviceNames = [], timeRange = '24h') {
    const services = Array.isArray(serviceNames) ? serviceNames.join(',') : serviceNames
    const response = await fetch(
      `${this.baseURL}/export/logs?services=${services}&range=${timeRange}`,
    )
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    
    return response.blob()
  }

  // Real-time data (WebSocket simulation with polling)
  startPolling(callback, interval = 30000) {
    const poll = async () => {
      try {
        const [stats, activity, threats] = await Promise.all([
          this.getDashboardStats(),
          this.getNetworkActivity(),
          this.getTopThreats(),
        ])
        
        callback({
          stats,
          activity,
          threats,
          timestamp: new Date().toISOString(),
        })
      } catch (error) {
        console.error('Polling error:', error)
      }
    }

    // Initial call
    poll()
    
    // Set up interval
    const intervalId = setInterval(poll, interval)
    
    // Return cleanup function
    return () => clearInterval(intervalId)
  }
}

// Create singleton instance
const apiService = new ApiService()

export default apiService
