import { useState, useEffect } from 'react'
import { 
  Activity, 
  Database, 
  AlertTriangle,
  RefreshCw,
  Download,
  Settings,
} from 'lucide-react'

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
  const [useCompactTitle, setUseCompactTitle] = useState(false)

  // Check viewport width to determine which ASCII art to show
  useEffect(() => {
    const checkViewportWidth = () => {
      setUseCompactTitle(window.innerWidth < 1200)
    }

    checkViewportWidth()
    window.addEventListener('resize', checkViewportWidth)
    
    return () => window.removeEventListener('resize', checkViewportWidth)
  }, [])

  // Mock API service functions
  const fetchDashboardData = async () => {
    setIsLoading(true)
    try {
      await new Promise(resolve => setTimeout(resolve, 1000))
      
      setDashboardData(prev => ({
        ...prev,
        packetsPerHour: Math.floor(Math.random() * 10000) + 1000,
        cpuUsage: Math.floor(Math.random() * 40) + 40,
        memoryUsage: Math.floor(Math.random() * 30) + 30,
        activeConnections: Math.floor(Math.random() * 500) + 100,
      }))

      setNetworkData(prev => 
        prev.map(item => ({
          ...item,
          packets: Math.floor(Math.random() * 8000) + 1000,
          threats: Math.floor(Math.random() * 15) + 1,
        }))
      )

    } catch (error) {
      console.error('Failed to fetch dashboard data:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const handleRefresh = () => {
    fetchDashboardData()
  }

  const handleExport = () => {
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

  // Simple Bar Chart Component
  const SimpleBarChart = ({ data }) => {
    const maxValue = Math.max(...data.map(d => d.threats))
    
    return (
      <div className="h-full flex items-end justify-between space-x-2 px-4">
        {data.map((item, index) => (
          <div key={index} className="flex flex-col items-center space-y-2 flex-1">
            <div 
              className="bg-red-500 w-full min-w-8 rounded-t transition-all duration-300 hover:bg-red-400"
