import { useState, useEffect } from 'react'
import { 
  Activity, 
  Database, 
  AlertTriangle,
  RefreshCw,
  Download,
  Settings,
} from 'lucide-react'
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, BarChart, Bar } from 'recharts'
// Import the API service (you can uncomment this when you have it set up)
// import apiService from '../services/apiService'

const Dashboard = () => {
  const [dashboardData, setDashboardData] = useState({
    packetsPerHour: 4516,
    cpuUsage: 67,
    memoryUsage: 45,
    databaseStorage: 78,
    activeConnections: 234,
    suspectIPs: [],
    mostAttackedService: 'HTTP',
    recentActivity: [],
  })

  const [networkData, setNetworkData] = useState([
    { time: '00:00', packets: 2400, threats: 5 },
    { time: '04:00', packets: 1398, threats: 2 },
    { time: '08:00', packets: 9800, threats: 8 },
    { time: '12:00', packets: 3908, threats: 12 },
    { time: '16:00', packets: 4800, threats: 6 },
    { time: '20:00', packets: 3800, threats: 4 },
  ])

  const [topThreats, setTopThreats] = useState([
    { ip: '203.0.113.45', detections: 23, country: 'Unknown' },
    { ip: '198.51.100.78', detections: 18, country: 'CN' },
    { ip: '192.0.2.123', detections: 15, country: 'RU' },
    { ip: '10.0.0.89', detections: 12, country: 'Local' },
  ])

  const [isLoading, setIsLoading] = useState(false)

  // Mock API service functions (replace with actual API calls)
  const fetchDashboardData = async () => {
    setIsLoading(true)
    try {
      // Future: Replace with actual API call
      // const data = await apiService.getDashboardStats()
      // setDashboardData(data)
      
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 1000))
      
      // Mock data update
      setDashboardData(prev => ({
        ...prev,
        packetsPerHour: Math.floor(Math.random() * 1000) + 4000,
        cpuUsage: Math.floor(Math.random() * 40) + 40,
        memoryUsage: Math.floor(Math.random() * 30) + 35,
        databaseStorage: Math.floor(Math.random() * 20) + 70,
        activeConnections: Math.floor(Math.random() * 100) + 200,
      }))
    } catch (error) {
      console.error('Error fetching dashboard data:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const fetchNetworkActivity = async () => {
    try {
      // Future: Replace with actual API call
      // const data = await apiService.getNetworkActivity()
      // setNetworkData(data)
      
      // Simulate real-time data updates
      const newData = networkData.map(item => ({
        ...item,
        packets: Math.floor(Math.random() * 8000) + 1000,
        threats: Math.floor(Math.random() * 15) + 1,
      }))
      setNetworkData(newData)
    } catch (error) {
      console.error('Error fetching network data:', error)
    }
  }

  const fetchTopThreats = async () => {
    try {
      // Future: Replace with actual API call
      // const data = await apiService.getTopThreats()
      // setTopThreats(data)
      
      // Simulate threat data updates
      const updatedThreats = topThreats.map(threat => ({
        ...threat,
        detections: Math.floor(Math.random() * 30) + 5,
      }))
      setTopThreats(updatedThreats)
    } catch (error) {
      console.error('Error fetching threats data:', error)
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

  const handleRefresh = () => {
    fetchDashboardData()
    fetchNetworkActivity()
    fetchTopThreats()
  }

  const handleExport = () => {
    console.log('Exporting data...')
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
          <span className="text-white text-xl font-bold">{dashboardData.packetsPerHour}</span>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-100 p-4">
      <div className="max-w-full mx-auto px-4">
        {/* Header - Fixed height with consistent layout */}
        <div className="grid grid-cols-12 gap-4 mb-6">
          {/* ASCII Art Container - Fixed height */}
          <div className="col-span-12 md:col-span-7 lg:col-span-8 xl:col-span-9 bg-gray-600 text-white p-4 rounded-lg overflow-hidden">
            <div className="flex items-start justify-center min-h-20">
              <div className="flex-shrink-0 w-full flex justify-left">
                {/* Single line ASCII art for larger screens */}
                <div className="font-mono text-white leading-none overflow-hidden hidden xl:block">
                  <pre className="whitespace-pre text-[0.6rem] lg:text-[0.7rem] xl:text-xs">
                    {`██████╗  ██████╗  ██████╗ █████╗ ██╗         ██╗███╗   ███╗██╗███████╗██╗     
██╔══██╗██╔═████╗██╔════╝██╔══██╗██║        ██╔╝████╗ ████║██║██╔════╝██║     
██████╔╝██║██╔██║██║     ███████║██║       ██╔╝ ██╔████╔██║██║█████╗  ██║     
██╔══██╗████╔╝██║██║     ██╔══██║██║      ██╔╝  ██║╚██╔╝██║██║██╔══╝  ██║     
██████╔╝╚██████╔╝╚██████╗██║  ██║███████╗██╔╝   ██║ ╚═╝ ██║██║███████╗███████╗
╚═════╝  ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝`}
                  </pre>
                </div>

                {/* Compact ASCII art for medium to large screens */}
                <div className="font-mono text-white leading-none overflow-hidden hidden md:block xl:hidden">
                  <pre className="whitespace-pre text-[0.35rem] lg:text-[0.4rem]">
                    {`██████╗  ██████╗  ██████╗ █████╗ ██╗         ██╗███╗   ███╗██╗███████╗██╗     
██╔══██╗██╔═████╗██╔════╝██╔══██╗██║        ██╔╝████╗ ████║██║██╔════╝██║     
██████╔╝██║██╔██║██║     ███████║██║       ██╔╝ ██╔████╔██║██║█████╗  ██║     
██╔══██╗████╔╝██║██║     ██╔══██║██║      ██╔╝  ██║╚██╔╝██║██║██╔══╝  ██║     
██████╔╝╚██████╔╝╚██████╗██║  ██║███████╗██╔╝   ██║ ╚═╝ ██║██║███████╗███████╗
╚═════╝  ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝`}
                  </pre>
                </div>

                {/* Compact two-line ASCII art for smaller screens - positioned at top */}
                <div className="font-mono text-white leading-none overflow-hidden block md:hidden">
                  <pre className="whitespace-pre text-[0.3rem] sm:text-[0.35rem]">
                    {`██████╗  ██████╗  ██████╗ █████╗ ██╗             
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
            ╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝`}
                  </pre>
                </div>
              </div>
            </div>
          </div>

          {/* Buttons Container - Fixed height */}
          <div className="col-span-12 md:col-span-5 lg:col-span-4 xl:col-span-3 bg-gray-600 text-white p-2 md:p-3 lg:p-4 rounded-lg">
            <div className="flex items-center justify-center h-20">
              <div className="flex flex-col md:flex-row space-y-1 md:space-y-0 md:space-x-1 lg:space-x-2 flex-shrink-0">
                <button 
                  onClick={handleRefresh}
                  className="dashboard-button flex items-center justify-center space-x-1 text-sm px-2 py-1 md:px-3 md:py-2"
                  disabled={isLoading}
                >
                  <RefreshCw className={`h-3 w-3 md:h-4 md:w-4 ${isLoading ? 'animate-spin' : ''}`} />
                  <span className="hidden lg:inline">Refresh</span>
                </button>
                <button 
                  onClick={handleExport}
                  className="dashboard-button flex items-center justify-center space-x-1 text-sm px-2 py-1 md:px-3 md:py-2"
                >
                  <Download className="h-3 w-3 md:h-4 md:w-4" />
                  <span className="hidden lg:inline">Export</span>
                </button>
                <button className="dashboard-button flex items-center justify-center px-2 py-1 md:px-3 md:py-2">
                  <Settings className="h-3 w-3 md:h-4 md:w-4" />
                </button>
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
                <CircularProgress value={75} />
              </div>
              <p className="text-center text-orange-400">packet/hr</p>
            </div>

            {/* Top Suspect IPs */}
            <div className="dashboard-card p-4">
              <h3 className="text-lg font-semibold mb-4">Top suspect IPs</h3>
              <div className="space-y-2">
                <div className="flex justify-between text-sm font-semibold border-b border-gray-600 pb-2">
                  <span>IP address</span>
                  <span># of detections</span>
                </div>
                {topThreats.map((threat, index) => (
                  <div key={index} className="threat-row">
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
              <h3 className="text-lg font-semibold mb-4">CPU usage</h3>
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
              <h3 className="text-lg font-semibold mb-4">Multi purpose dashboard</h3>
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
                <div className="text-white text-xs mb-1">Memory</div>
                <div className="text-orange-400 text-sm font-bold">
                  {dashboardData.memoryUsage}%
                </div>
              </div>
              <div className="metric-card">
                <div className="text-white text-xs mb-1">Storage</div>
                <div className="text-orange-400 text-sm font-bold">
                  {dashboardData.databaseStorage}%
                </div>
              </div>
              <div className="metric-card">
                <div className="text-white text-xs mb-1">Uptime</div>
                <div className="text-orange-400 text-sm font-bold">24h</div>
              </div>
            </div>

            {/* Most Attacked Service */}
            <div className="dashboard-card p-4">
              <h4 className="font-semibold mb-2">Most attacked service</h4>
              <div className="text-2xl text-orange-400 font-bold mb-1">
                {dashboardData.mostAttackedService}
              </div>
              <div className="text-sm text-gray-400">Port 80/443</div>
              <div className="mt-2 text-xs text-gray-500">
                {Math.floor(Math.random() * 100) + 50} attacks today
              </div>
            </div>

            {/* Utils Buttons */}
            <div className="space-y-2">
              {[
                { name: 'Export', icon: Download },
                { name: 'Refresh', icon: RefreshCw },
                { name: 'Settings', icon: Settings },
                { name: 'Alerts', icon: AlertTriangle },
                { name: 'Reports', icon: Database },
                { name: 'Activity', icon: Activity },
              ].map((item, index) => {
                const Icon = item.icon
                return (
                  <button
                    key={index}
                    className="w-full dashboard-card p-2 flex items-center space-x-2 hover:bg-gray-800"
                    onClick={item.name === 'Export' ? handleExport : item.name === 'Refresh' ? handleRefresh : undefined}
                  >
                    <Icon className="h-4 w-4" />
                    <span>{item.name}</span>
                  </button>
                )
              })}
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
            <div className="text-sm">
              CPU: {dashboardData.cpuUsage}%
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
