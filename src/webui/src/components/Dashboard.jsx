import { useState, useEffect } from 'react'
import { 
  Download,
} from 'lucide-react'
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, BarChart, Bar } from 'recharts'
import apiService from '../services/apiService.js'

const Dashboard = () => {
  const [dashboardData, setDashboardData] = useState({
    packetsPerHour: 0,
    cpuUsage: 67,
    memoryUsage: 45,
    databaseStorage: 78,
    activeConnections: 234,
    suspectIPs: [],
    mostAttackedService: 'HTTP',
    recentActivity: [],
  })
  
  // Uncomment to use the fetchNetworkActivity 
  // const networkData = fetchNetworkActivity
  const [networkData, setNetworkData] = useState([
    { time: '00:00', packets: 2400, threats: 5 },
    { time: '04:00', packets: 1398, threats: 2 },
    { time: '08:00', packets: 9800, threats: 8 },
    { time: '12:00', packets: 3908, threats: 12 },
    { time: '16:00', packets: 4800, threats: 6 },
    { time: '20:00', packets: 3800, threats: 4 },
  ])

  const fetchNetworkActivity = async () => {
    try {
      const data = await apiService.getByteTransferTimeline()
      setNetworkData(data)
    } catch (error) {
      console.error('Error fetching network data:', error)
    }
  }

  // Uncomment to use the fetchTopThreats
  // const topThreats = fetchTopThreats
  const [topThreats, setTopThreats] = useState([
    { ip: '203.0.113.45', detections: 23 },
    { ip: '198.51.100.78', detections: 18 },
    { ip: '192.0.2.123', detections: 15 },
    { ip: '10.0.0.89', detections: 12 },
  ])

  const fetchTopThreats = async () => {
    try {
      const data = await apiService.getClientAddressesOrdered()
      setTopThreats(data)
      
    } catch (error) {
      console.error('Error fetching threats data:', error)
    }
  }


  // Mock API service functions (replace with actual API calls)
  const fetchDashboardData = async () => {
    try {
      const avgPacketPerHour = await apiService.getAvgPacketPerHour()
      const activeConnections = await apiService.getNumberOfActiveSessions()
      const mostAttackedService = await apiService.getMostAttackedService()
      
      // Mock data update
      setDashboardData(prev => ({
        ...prev,
        packetsPerHour: avgPacketPerHour,
        cpuUsage: Math.floor(Math.random() * 40) + 40,
        memoryUsage: Math.floor(Math.random() * 30) + 35,
        databaseStorage: Math.floor(Math.random() * 20) + 70,
        activeConnections,
        mostAttackedService,
      }))
    } catch (error) {
      console.error('Error fetching dashboard data:', error)
    }   
  }

  useEffect(() => {
    fetchDashboardData()
    fetchNetworkActivity()
    fetchTopThreats()

    // Set up polling for real-time updates
    const interval = setInterval(() => {
      fetchDashboardData()
      fetchNetworkActivity()
    }, 30000) // Update every 30 seconds

    return () => clearInterval(interval)
  }, [])


  const handleExport = () => {
    // Implement export functionality
    const dataToExport = {
      timestamp: new Date().toISOString(),
      dashboardData,
      networkData,
      topThreats,
    }
    
    const dataStr = JSON.stringify(dataToExport, null, 2)
    const dataBlob = new Blob([dataStr], { type: 'application/json' })
    const url = URL.createObjectURL(dataBlob)
    const link = document.createElement('a')
    link.href = url
    link.download = `miel-report-${Date.now()}.json`
    link.click()
  }

  // Circular progress component
  const CircularProgress = ({ value, size = 96, strokeWidth = 8 }) => {
    const radius = (size - strokeWidth) / 2
    const circumference = 2 * Math.PI * radius
    const strokeDasharray = circumference
    const strokeDashoffset = circumference - (value / 100) * circumference

    return (
      <div className="relative" style={{ width: size, height: size }}>
        <svg
          width={size}
          height={size}
          className="transform -rotate-90"
        >
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            stroke="#252a33"
            strokeWidth={strokeWidth}
            fill="none"
          />
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            stroke="#eea537"
            strokeWidth={strokeWidth}
            fill="none"
            strokeDasharray={strokeDasharray}
            strokeDashoffset={strokeDashoffset}
            strokeLinecap="round"
            className="transition-all duration-300"
          />
        </svg>
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-white text-xl font-mono">{dashboardData.packetsPerHour}</span>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-100 p-4">
      <div className="max-w-full mx-auto px-4">
        {/* Header - Fixed height with consistent layout */}
        <div className="flex gap-4 mb-6 h-24 min-h-24">
          {/* ASCII Art Container - Flex-grow to take remaining space */}
          <div className="flex-grow bg-gray-600 text-white p-4 rounded-lg overflow-hidden min-w-0">
            <div className="flex items-center justify-center h-full">
              <div className="w-full flex justify-left">
                {/* Single line ASCII art for larger screens */}
                <div className="font-mono text-white leading-none overflow-hidden hidden xl:block">
                  <pre className="whitespace-pre text-[0.6rem] lg:text-[0.7rem] xl:text-[0.8rem]">
                    {`██████╗  ██████╗  ██████╗ █████╗ ██╗         ██╗███╗   ███╗██╗███████╗██╗     
██╔══██╗██╔═████╗██╔════╝██╔══██╗██║        ██╔╝████╗ ████║██║██╔════╝██║     
██████╔╝██║██╔██║██║     ███████║██║       ██╔╝ ██╔████╔██║██║█████╗  ██║     
██╔══██╗████╔╝██║██║     ██╔══██║██║      ██╔╝  ██║╚██╔╝██║██║██╔══╝  ██║     
██████╔╝╚██████╔╝╚██████╗██║  ██║███████╗██╔╝   ██║ ╚═╝ ██║██║███████╗███████╗
╚═════╝  ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝`}
                  </pre>
                </div>

                {/* Compact ASCII art for medium to large screens */}
                <div className="font-mono text-white leading-none overflow-hidden hidden lg:block xl:hidden">
                  <pre className="whitespace-pre text-[0.45rem]">
                    {`██████╗  ██████╗  ██████╗ █████╗ ██╗         ██╗███╗   ███╗██╗███████╗██╗     
██╔══██╗██╔═████╗██╔════╝██╔══██╗██║        ██╔╝████╗ ████║██║██╔════╝██║     
██████╔╝██║██╔██║██║     ███████║██║       ██╔╝ ██╔████╔██║██║█████╗  ██║     
██╔══██╗████╔╝██║██║     ██╔══██║██║      ██╔╝  ██║╚██╔╝██║██║██╔══╝  ██║     
██████╔╝╚██████╔╝╚██████╗██║  ██║███████╗██╔╝   ██║ ╚═╝ ██║██║███████╗███████╗
╚═════╝  ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝`}
                  </pre>
                </div>

                {/* Medium screens */}
                <div className="font-mono text-white leading-none overflow-hidden hidden md:block lg:hidden">
                  <pre className="whitespace-pre text-[0.35rem]">
                    {`
██████╗  ██████╗  ██████╗ █████╗ ██╗             
██╔══██╗██╔═████╗██╔════╝██╔══██╗██║             
██████╔╝██║██╔██║██║     ███████║██║             
██╔══██╗████╔╝██║██║     ██╔══██║██║             
██████╔╝╚██████╔╝╚██████╗██║  ██║███████╗        
╚═════╝  ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝        
                                                 
                ██╗███╗   ███╗██╗███████╗██╗     
               ██╔╝████╗ ████║██║██╔════╝██║     
              ██╔╝ ██╔████╔██║██║█████╗  ██║     
             ██╔╝  ██║╚██╔╝██║██║██╔══╝  ██║     
            ██╔╝   ██║ ╚═╝ ██║██║███████╗███████╗
            ╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝
`}
                  </pre>
                </div>

                {/* Small screens - Very compact */}
                <div className="font-mono text-white leading-none overflow-hidden block md:hidden">
                  <pre className="whitespace-pre text-[0.25rem] sm:text-[0.3rem]">
                    {`MIEL - Monitoring Infrastructure 
for Enhanced Learning`}
                  </pre>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-12 gap-4">
          {/* Left Column */}
          <div className="col-span-12 md:col-span-3 space-y-4">
            {/* Packets per Hour */}
            <div className="dashboard-card p-6">
              <div className="flex justify-center mb-4">
                <CircularProgress value={dashboardData.packetsPerHour || 0} />
              </div>
              <p className="text-center text-orange-400">packet/hr</p>
            </div>

            {/* Top Suspect IPs */}
            <div className="dashboard-card p-4">
              <h3 className="text-lg font-mono mb-4">Top suspect IPs</h3>
              <div className="space-y-2">
                <div className="flex justify-between text-sm font-mono border-b border-gray-600 pb-2">
                  <span>IP address</span>
                  <span># of detections</span>
                </div>
                {topThreats.map((threat) => (
                  <div key={threat.ip} className="threat-row">
                    <span className="text-orange-400">{threat.ip}</span>
                    <span>{threat.detections}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Center Column */}
          <div className="col-span-12 md:col-span-6 space-y-4">
            {/* CPU Usage Chart */}
            <div className="dashboard-card p-4">
              <h3 className="text-lg font-mono mb-4">Network Activity</h3>
              <div className="h-48">
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={networkData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#ccc" />
                    <YAxis stroke="#ccc" />
                    <Tooltip 
                      contentStyle={{ backgroundColor: '#333', border: 'none', borderRadius: '8px' }}
                      labelStyle={{ color: '#fff' }}
                    />
                    <Line 
                      type="monotone" 
                      dataKey="packets" 
                      stroke="#EE4537" 
                      strokeWidth={2}
                      dot={{ fill: '#EE4537', strokeWidth: 2 }}
                      activeDot={{ r: 6, fill: '#EE4537' }}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* Multi-purpose Dashboard */}
            <div className="dashboard-card p-4">
              <h3 className="text-lg font-mono mb-4">Possible detected threats</h3>
              <div className="h-64">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={networkData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#ccc" />
                    <YAxis stroke="#ccc" />
                    <Tooltip 
                      contentStyle={{ backgroundColor: '#333', border: 'none', borderRadius: '8px' }}
                      labelStyle={{ color: '#fff' }}
                    />
                    <Bar dataKey="threats" fill="#EE4537" radius={[4, 4, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </div>
          </div>

          {/* Right Column */}
          <div className="col-span-12 md:col-span-3 space-y-4">
            {/* System Indicators */}
            <div className="grid grid-cols-3 gap-2">
              <div className="metric-card">
                <div className="text-white text-xs xs:text-[0.2rem] mb-1">Memory</div>
                <div className="text-orange-400 text-sm font-mono">
                  {dashboardData.memoryUsage}%
                </div>
              </div>
              <div className="metric-card">
                <div className="text-white text-xs mb-1">Storage</div>
                <div className="text-orange-400 text-sm font-mono">
                  {dashboardData.databaseStorage}%
                </div>
              </div>
              <div className="metric-card">
                <div className="text-white text-xs mb-1">Uptime</div>
                <div className="text-orange-400 text-sm font-mono">24h</div>
              </div>
            </div>

            {/* Most Attacked Service */}
            <div className="dashboard-card p-4">
              <h4 className="font-mono mb-2">Most attacked service</h4>
              <div className="text-2xl text-orange-400 font-mono mb-1">
                {dashboardData.mostAttackedService.service || 'HTTP'}
              </div>
              <div className="mt-2 text-xs text-gray-500">
                {dashboardData.mostAttackedService.count || '0'} attacks today
              </div>
            </div>

            {/* Utils Buttons */}
            <div className="space-y-2">
              <button
                className="w-full dashboard-card p-2 flex items-center space-x-2 hover:bg-gray-800"
                onClick={handleExport}
              >
                <Download className="h-4 w-4" />
                <span>Export</span>
              </button>
            </div>
          </div>
        </div>

        {/* Status Footer */}
        <div className="mt-6 bg-gray-600 text-white p-3 rounded-lg flex flex-col md:flex-row justify-between items-center space-y-2 md:space-y-0">
          <div className="flex items-center space-x-4">
            <div className="flex items-center space-x-2">
              <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
              <span className="text-sm">System Online</span>
            </div>
            <div className="text-sm">
              Active Sessions: {dashboardData.activeConnections}
            </div>
          </div>
          <div className="text-sm">
            Last Update: {new Date().toLocaleTimeString()}
          </div>
        </div>
      </div>
    </div>
  )
}

export default Dashboard
