import {
  isPermissionGranted,
  onAction,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'
import { isDesktopRuntime } from '@/services/desktop'
import type { JobNotice } from '@/utils/jobNotifications'

export type DesktopNotificationPermission = 'granted' | 'not-granted' | 'unavailable'

export async function desktopNotificationPermission(): Promise<DesktopNotificationPermission> {
  if (!isDesktopRuntime()) return 'unavailable'
  try {
    return (await isPermissionGranted()) ? 'granted' : 'not-granted'
  } catch {
    return 'unavailable'
  }
}

export async function requestDesktopNotificationPermission() {
  if (!isDesktopRuntime()) return 'unavailable' as const
  try {
    if (await isPermissionGranted()) return 'granted' as const
    return (await requestPermission()) === 'granted'
      ? ('granted' as const)
      : ('not-granted' as const)
  } catch {
    return 'unavailable' as const
  }
}

export async function sendJobSystemNotification(notice: JobNotice) {
  if ((await desktopNotificationPermission()) !== 'granted') return false
  try {
    sendNotification({
      title: notice.title,
      body: notice.body,
      autoCancel: true,
      extra: { jobId: notice.jobId },
    })
    return true
  } catch {
    return false
  }
}

export async function sendTestDesktopNotification() {
  if ((await desktopNotificationPermission()) !== 'granted') return false
  try {
    sendNotification({
      title: 'Beatforge notifications are ready',
      body: 'Background job updates will appear here.',
      autoCancel: true,
    })
    return true
  } catch {
    return false
  }
}

export async function onDesktopNotificationAction(handler: (jobId: string) => void) {
  if (!isDesktopRuntime()) return () => undefined
  try {
    const listener = await onAction((event) => {
      if (event.actionId !== 'tap') return
      const jobId = event.notification?.extra.jobId
      if (typeof jobId === 'string' && jobId) handler(jobId)
    })
    return () => {
      void listener.unregister()
    }
  } catch {
    return () => undefined
  }
}
