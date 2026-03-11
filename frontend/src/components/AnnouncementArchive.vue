<template>
  <div class="announcement-archive">
    <h1>Announcement Archive</h1>
    <p class="intro">
      This page contains a public archive of all announcement emails sent to PluralSync users.
    </p>

    <div v-if="announcements.length === 0" class="no-announcements">
      <p>No announcements have been made yet.</p>
    </div>

    <div v-else class="announcements-list">
      <div v-for="announcement in announcements" :key="announcement.email_id" class="announcement">
        <h2>{{ announcement.subject }}</h2>
        <p class="date">{{ formatDate(announcement.date) }}</p>
        <div class="body">
          <pre>{{ announcement.body }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'

interface RenderedAnnouncement {
  email_id: string
  date: string
  subject: string
  body: string
}

const announcements = ref<RenderedAnnouncement[]>([])

function formatDate(dateStr: string): string {
  try {
    const date = new Date(dateStr + 'T00:00:00Z')
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    })
  } catch {
    return dateStr
  }
}

onMounted(async () => {
  try {
    const response = await fetch('/announcements.json')
    if (response.ok) {
      announcements.value = await response.json()
    }
  } catch (error) {
    console.error('Failed to load announcements:', error)
  }
})
</script>

<style scoped>
.announcement-archive {
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem 1rem;
}

h1 {
  color: var(--color-primary);
  margin-bottom: 1rem;
}

.intro {
  color: #666;
  margin-bottom: 2rem;
  line-height: 1.6;
}

.no-announcements {
  text-align: center;
  padding: 3rem;
  color: #999;
}

.announcements-list {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.announcement {
  background-color: #f8f9fa;
  border: 1px solid #dee2e6;
  border-radius: 0.5em;
  padding: 1.5rem;
}

.announcement h2 {
  color: #333;
  margin-top: 0;
  margin-bottom: 0.5rem;
  font-size: 1.4em;
}

.date {
  font-size: 0.9em;
  color: #888;
  margin-bottom: 1rem;
  font-style: italic;
}

.body {
  background-color: white;
  border: 1px solid #e9ecef;
  border-radius: 0.25em;
  padding: 1rem;
}

.body pre {
  white-space: pre-wrap;
  word-wrap: break-word;
  margin: 0;
  font-family: inherit;
  line-height: 1.6;
  color: #333;
}
</style>
