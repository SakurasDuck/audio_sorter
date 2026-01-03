pub const HTML_CONTENT: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Audio Sorter Dashboard</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://unpkg.com/vue@3/dist/vue.global.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body class="bg-gray-100 text-gray-800">
    <div id="app" class="min-h-screen p-8">
        <header class="mb-8 flex justify-between items-center bg-white p-4 rounded-lg shadow">
            <div>
                <h1 class="text-3xl font-bold text-indigo-600">Audio Library Dashboard</h1>
                <div class="text-sm text-gray-500 mt-1">
                    Loaded {{ tracks.length }} tracks
                </div>
            </div>
            <div class="flex space-x-4">
                <button 
                    @click="activeTab = 'library'" 
                    class="px-4 py-2 rounded font-medium"
                    :class="activeTab === 'library' ? 'bg-indigo-100 text-indigo-700' : 'text-gray-600 hover:bg-gray-50'">
                    Library
                </button>
                <button 
                    @click="activeTab = 'duplicates'" 
                    class="px-4 py-2 rounded font-medium"
                    :class="activeTab === 'duplicates' ? 'bg-indigo-100 text-indigo-700' : 'text-gray-600 hover:bg-gray-50'">
                    Duplicates ({{ duplicateGroups.length }})
                </button>
                <div class="border-l pl-4"></div>
                <button 
                    @click="startScan" 
                    :disabled="isScanning"
                    class="bg-indigo-600 text-white px-4 py-2 rounded hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center">
                    <span v-if="isScanning" class="mr-2 animate-spin">⟳</span>
                    {{ isScanning ? 'Scanning...' : 'Scan Library' }}
                </button>
            </div>
        </header>

        <!-- Scan Status Panel -->
        <div v-if="isScanning || scanStatus.elapsed_secs > 0" class="bg-white p-6 rounded-lg shadow mb-8 border-l-4 border-indigo-500">
            <h2 class="text-lg font-bold mb-4 flex justify-between">
                <span>Scan Progress</span>
                <span class="text-sm font-normal text-gray-500">Elapsed: {{ formatTime(scanStatus.elapsed_secs) }}</span>
            </h2>
            
            <div class="mb-4">
                <div class="flex justify-between text-sm mb-1">
                    <span>Processed: {{ scanStatus.files_processed }} / {{ scanStatus.files_total || '?' }}</span>
                    <span>Errors: {{ scanStatus.errors }}</span>
                </div>
                <!-- Progress Bar -->
                <div class="w-full bg-gray-200 rounded-full h-2.5">
                    <div class="bg-indigo-600 h-2.5 rounded-full transition-all duration-500" 
                         :style="{ width: percentComplete + '%' }"></div>
                </div>
                <div class="text-xs text-gray-500 mt-1 truncate">
                    Currently: {{ scanStatus.current_file }}
                </div>
            </div>

            <!-- Resource Monitor -->
            <div class="grid grid-cols-2 gap-4">
                <div class="bg-gray-50 p-3 rounded">
                    <span class="text-xs text-gray-500 uppercase">CPU Usage</span>
                    <div class="text-xl font-mono">{{ scanStatus.resources.cpu_usage.toFixed(1) }}%</div>
                    <div class="w-full bg-gray-200 h-1 mt-1 rounded">
                         <div class="bg-green-500 h-1 rounded transition-all duration-500" :style="{ width: Math.min(scanStatus.resources.cpu_usage, 100) + '%' }"></div>
                    </div>
                </div>
                <div class="bg-gray-50 p-3 rounded">
                    <span class="text-xs text-gray-500 uppercase">Memory Usage</span>
                    <div class="text-xl font-mono">{{ formatBytes(scanStatus.resources.memory_usage) }}</div>
                </div>
            </div>
        </div>

        <!-- Library View -->
        <div v-show="activeTab === 'library'">
            <!-- Stats Cards -->
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
                <div class="bg-white p-6 rounded-lg shadow">
                    <h3 class="text-gray-500 text-sm font-uppercase">Total Tracks</h3>
                    <p class="text-4xl font-bold mt-2">{{ tracks.length }}</p>
                </div>
                <div class="bg-white p-6 rounded-lg shadow">
                    <h3 class="text-gray-500 text-sm font-uppercase">Total Library Size</h3>
                    <p class="text-4xl font-bold mt-2">{{ formatBytes(totalSize) }}</p>
                </div>
                 <div class="bg-white p-6 rounded-lg shadow">
                    <h3 class="text-gray-500 text-sm font-uppercase">Unique Artists</h3>
                    <p class="text-4xl font-bold mt-2">{{ uniqueArtists }}</p>
                </div>
            </div>

            <!-- Search Bar -->
            <div class="bg-white p-4 rounded-lg shadow mb-6">
                <input 
                    v-model="searchQuery" 
                    type="text" 
                    placeholder="Search by artist, title, or album..." 
                    class="w-full p-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
                >
            </div>

            <!-- Data Table -->
            <div class="bg-white rounded-lg shadow overflow-hidden">
                <table class="min-w-full leading-normal">
                    <thead>
                        <tr>
                            <th class="px-5 py-3 border-b-2 border-gray-200 bg-gray-50 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider">Title</th>
                            <th class="px-5 py-3 border-b-2 border-gray-200 bg-gray-50 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider">Artist</th>
                            <th class="px-5 py-3 border-b-2 border-gray-200 bg-gray-50 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider">Album</th>
                            <th class="px-5 py-3 border-b-2 border-gray-200 bg-gray-50 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider">Original Artist</th>
                            <th class="px-5 py-3 border-b-2 border-gray-200 bg-gray-50 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider">Size</th>
                            <th class="px-5 py-3 border-b-2 border-gray-200 bg-gray-50 text-center text-xs font-semibold text-gray-600 uppercase tracking-wider">Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="track in filteredTracks" :key="track.path">
                            <td class="px-5 py-5 border-b border-gray-200 bg-white text-sm">
                                <div class="flex items-center">
                                    <div class="ml-3">
                                        <p class="text-gray-900 whitespace-no-wrap font-medium">
                                            {{ track.metadata.title || 'Unknown Title' }}
                                        </p>
                                        <p class="text-gray-400 text-xs">{{ track.path }}</p>
                                    </div>
                                </div>
                            </td>
                            <td class="px-5 py-5 border-b border-gray-200 bg-white text-sm">
                                <p class="text-gray-900 whitespace-no-wrap">{{ track.metadata.artist || 'Unknown Artist' }}</p>
                            </td>
                            <td class="px-5 py-5 border-b border-gray-200 bg-white text-sm">
                                <p class="text-gray-900 whitespace-no-wrap">{{ track.metadata.album || '-' }}</p>
                            </td>
                             <td class="px-5 py-5 border-b border-gray-200 bg-white text-sm">
                                <span v-if="track.metadata.original_artist" class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-100 text-green-800">
                                    {{ track.metadata.original_artist }}
                                </span>
                                 <span v-else class="text-gray-400">-</span>
                            </td>
                            <td class="px-5 py-5 border-b border-gray-200 bg-white text-sm">
                                <p class="text-gray-900 whitespace-no-wrap">{{ formatBytes(track.file_size) }}</p>
                            </td>
                            <td class="px-5 py-5 border-b border-gray-200 bg-white text-sm text-center">
                                <button @click="findSimilar(track)" class="bg-purple-500 hover:bg-purple-600 text-white text-xs px-3 py-1 rounded transition-colors" title="Find Similar Songs">
                                    🎵 Similar
                                </button>
                            </td>
                        </tr>
                    </tbody>
                </table>
                 <div v-if="filteredTracks.length === 0" class="p-4 text-center text-gray-500">
                    No tracks found matching your search.
                </div>
                 <div v-if="filteredTracks.length >= 100" class="p-2 text-center text-xs text-gray-400 bg-gray-50">
                    Showing first 100 matches ({{ filteredTracks.length }} total)
                </div>
            </div>
        </div>

        <!-- Duplicates View -->
        <div v-show="activeTab === 'duplicates'">
            <div v-if="duplicateGroups.length === 0" class="bg-white p-8 rounded-lg shadow text-center text-gray-500">
                <h3 class="text-xl font-medium">No Duplicates Found</h3>
                <p class="mt-2">Runs a scan to detect duplicate files based on audio fingerprints.</p>
            </div>
            
            <div v-else class="space-y-6">
                <div v-for="(group, idx) in duplicateGroups" :key="idx" class="bg-white rounded-lg shadow overflow-hidden">
                    <div class="bg-red-50 px-4 py-2 border-b border-red-100 flex justify-between items-center">
                        <span class="text-red-800 font-medium">Duplicate Group #{{ idx + 1 }}</span>
                        <span class="text-xs text-red-600 bg-red-100 px-2 py-1 rounded">{{ group.length }} files</span>
                    </div>
                    <table class="min-w-full">
                        <tbody>
                            <tr v-for="track in group" :key="track.path" class="border-b last:border-0 hover:bg-gray-50">
                                <td class="px-4 py-3 text-sm">
                                    <div class="font-medium">{{ track.metadata.title }}</div>
                                    <div class="text-xs text-gray-500">{{ track.path }}</div>
                                </td>
                                <td class="px-4 py-3 text-sm text-right">
                                    {{ track.metadata.artist }}
                                </td>
                                <td class="px-4 py-3 text-sm text-right text-gray-500">
                                    {{ formatBytes(track.file_size) }}
                                </td>
                                <td class="px-4 py-3 text-sm text-right text-gray-500">
                                    {{ formatTime(track.metadata.duration) }}
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>

        <!-- Recommendations Modal -->
        <div v-if="showRecommendModal" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" @click.self="showRecommendModal = false">
            <div class="bg-white rounded-lg shadow-xl w-full max-w-2xl max-h-[80vh] overflow-hidden">
                <div class="bg-purple-600 text-white px-6 py-4 flex justify-between items-center">
                    <h3 class="text-lg font-bold">🎵 Similar Songs</h3>
                    <button @click="showRecommendModal = false" class="text-white hover:text-gray-200 text-2xl">&times;</button>
                </div>
                <div class="p-4">
                    <div v-if="recommendLoading" class="text-center py-8">
                        <span class="animate-spin text-3xl">⟳</span>
                        <p class="mt-2 text-gray-500">Finding similar songs...</p>
                    </div>
                    <div v-else-if="recommendations.length === 0" class="text-center py-8 text-gray-500">
                        <p>No similar songs found. Try scanning with analysis enabled.</p>
                    </div>
                    <div v-else class="overflow-y-auto max-h-96">
                        <div class="mb-4 text-sm text-gray-600">
                            Based on: <strong>{{ recommendSourceTrack?.metadata?.title }}</strong> by {{ recommendSourceTrack?.metadata?.artist }}
                        </div>
                        <table class="w-full">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-4 py-2 text-left text-xs font-semibold text-gray-600">#</th>
                                    <th class="px-4 py-2 text-left text-xs font-semibold text-gray-600">Title</th>
                                    <th class="px-4 py-2 text-left text-xs font-semibold text-gray-600">Artist</th>
                                    <th class="px-4 py-2 text-right text-xs font-semibold text-gray-600">Similarity</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr v-for="(rec, idx) in recommendations" :key="rec.path" class="border-b hover:bg-gray-50">
                                    <td class="px-4 py-3 text-sm text-gray-500">{{ idx + 1 }}</td>
                                    <td class="px-4 py-3 text-sm font-medium">{{ rec.title }}</td>
                                    <td class="px-4 py-3 text-sm text-gray-600">{{ rec.artist }}</td>
                                    <td class="px-4 py-3 text-sm text-right">
                                        <span :class="getSimilarityClass(rec.distance)">{{ formatSimilarity(rec.distance) }}</span>
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </div>

    </div>

    <script>
        const { createApp, ref, computed, onMounted, watch } = Vue;

        createApp({
            setup() {
                const tracks = ref([]);
                const duplicateGroups = ref([]);
                const searchQuery = ref('');
                const activeTab = ref('library');

                // Scan State
                const isScanning = ref(false);
                const scanStatus = ref({
                    is_scanning: false,
                    files_total: 0,
                    files_processed: 0,
                    current_file: '',
                    elapsed_secs: 0,
                    resources: { cpu_usage: 0, memory_usage: 0 },
                    errors: 0
                });

                // Recommendations State
                const showRecommendModal = ref(false);
                const recommendLoading = ref(false);
                const recommendations = ref([]);
                const recommendSourceTrack = ref(null);

                const fetchTracks = async () => {
                    try {
                        const res = await fetch('/api/tracks');
                        const data = await res.json();
                        tracks.value = data;
                    } catch (e) {
                        console.error("Failed to load tracks", e);
                    }
                };
                
                const fetchDuplicates = async () => {
                     try {
                        const res = await fetch('/api/duplicates');
                        const data = await res.json();
                        duplicateGroups.value = data;
                    } catch (e) {
                        console.error("Failed to load duplicates", e);
                    }
                }

                const startScan = async () => {
                    try {
                        const res = await fetch('/api/scan/start', { method: 'POST' });
                        const data = await res.json();
                        if (data.status === 'started') {
                            isScanning.value = true;
                            pollStatus();
                        } else {
                            alert('Failed to start scan: ' + (data.error || 'Unknown error'));
                        }
                    } catch (e) {
                        alert('Error starting scan: ' + e);
                    }
                };

                const pollStatus = async () => {
                    const timer = setInterval(async () => {
                        try {
                            const res = await fetch('/api/scan/status');
                            const status = await res.json();
                            scanStatus.value = status;
                            isScanning.value = status.is_scanning;

                            if (!status.is_scanning) {
                                clearInterval(timer);
                                fetchTracks(); // Reload data
                                fetchDuplicates();
                            }
                        } catch (e) {
                            console.error("Polling error", e);
                        }
                    }, 1000);
                };

                const findSimilar = async (track) => {
                    recommendSourceTrack.value = track;
                    showRecommendModal.value = true;
                    recommendLoading.value = true;
                    recommendations.value = [];

                    try {
                        const res = await fetch(`/api/recommend?path=${encodeURIComponent(track.path)}`);
                        const data = await res.json();
                        if (data.error) {
                            console.error('Recommendation error:', data.error);
                            recommendations.value = [];
                        } else {
                            recommendations.value = data;
                        }
                    } catch (e) {
                        console.error('Failed to get recommendations', e);
                    } finally {
                        recommendLoading.value = false;
                    }
                };

                const formatSimilarity = (distance) => {
                    if (distance === 0) return '100%';
                    const similarity = Math.max(0, 100 - distance * 100);
                    return similarity.toFixed(0) + '%';
                };

                const getSimilarityClass = (distance) => {
                    if (distance < 0.1) return 'text-green-600 font-bold';
                    if (distance < 0.3) return 'text-green-500';
                    if (distance < 0.5) return 'text-yellow-600';
                    return 'text-gray-500';
                };

                onMounted(() => {
                    fetchTracks();
                    fetchDuplicates();
                    // Check if scan is already running on load
                    pollStatus();
                });

                const totalSize = computed(() => {
                    return tracks.value.reduce((acc, t) => acc + t.file_size, 0);
                });

                const uniqueArtists = computed(() => {
                    const artists = new Set(tracks.value.map(t => t.metadata.artist));
                    return artists.size;
                });

                const filteredTracks = computed(() => {
                    const q = searchQuery.value.toLowerCase();
                    if (!q) return tracks.value.slice(0, 100);
                    
                    return tracks.value.filter(t => {
                        const title = (t.metadata.title || '').toLowerCase();
                        const artist = (t.metadata.artist || '').toLowerCase();
                        const album = (t.metadata.album || '').toLowerCase();
                        return title.includes(q) || artist.includes(q) || album.includes(q);
                    }).slice(0, 100); 
                });
                
                const percentComplete = computed(() => {
                    if (!scanStatus.value.files_total) return 0;
                    return (scanStatus.value.files_processed / scanStatus.value.files_total) * 100;
                });

                const formatBytes = (bytes, decimals = 2) => {
                    if (!+bytes) return '0 Bytes';
                    const k = 1024;
                    const dm = decimals < 0 ? 0 : decimals;
                    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
                    const i = Math.floor(Math.log(bytes) / Math.log(k));
                    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
                }
                
                const formatTime = (secs) => {
                    if (!secs) return '0s';
                    const m = Math.floor(secs / 60);
                    const s = secs % 60;
                    return `${m}m ${s}s`;
                }

                return {
                    tracks,
                    duplicateGroups,
                    searchQuery,
                    activeTab,
                    isScanning,
                    scanStatus,
                    filteredTracks,
                    totalSize,
                    uniqueArtists,
                    formatBytes,
                    formatTime,
                    startScan,
                    findSimilar,
                    showRecommendModal,
                    recommendLoading,
                    recommendations,
                    recommendSourceTrack,
                    formatSimilarity,
                    getSimilarityClass,
                    percentComplete
                };
            }
        }).mount('#app');
    </script>
</body>
</html>
"#;
