import { lazy, Suspense, useEffect } from 'react'
import { Link, NavLink as RouterNavLink, Outlet, useLocation, useNavigate } from 'react-router'
import {
  ActionIcon,
  AppShell,
  Avatar,
  Box,
  Burger,
  Button,
  Divider,
  Group,
  Menu,
  NavLink,
  Stack,
  Text,
  Tooltip,
  UnstyledButton,
  useMantineColorScheme,
} from '@mantine/core'
import { useDisclosure } from '@mantine/hooks'
import type { Icon } from '@tabler/icons-react'
import {
  IconCalendarEvent,
  IconCircleCheck,
  IconInbox,
  IconLayoutDashboard,
  IconList,
  IconLogout,
  IconPlugConnected,
  IconPlus,
  IconSettings,
  IconSun,
  IconUser,
} from '@tabler/icons-react'
import { useQueryClient } from '@tanstack/react-query'

import { Brand } from './components/Brand'
import { ThemeToggle } from './components/ThemeToggle'
import { useCurrentUser } from './api/auth'
import { useSettings } from './api/settings'
import { clearToken } from './auth/token'
import { initials } from './lib/initials'

// Quick capture owns a date-time picker, so load it only when a user opens it.
const QuickCapture = lazy(() =>
  import('./components/QuickCapture').then((module) => ({
    default: module.QuickCapture,
  })),
)

interface NavItem {
  to: string
  label: string
  icon: Icon
}

const primaryNav: NavItem[] = [
  { to: '/dashboard', label: 'Dashboard', icon: IconLayoutDashboard },
  { to: '/inbox', label: 'Inbox', icon: IconInbox },
  { to: '/today', label: 'Today', icon: IconSun },
  { to: '/upcoming', label: 'Upcoming', icon: IconCalendarEvent },
  { to: '/items', label: 'All items', icon: IconList },
  { to: '/completed', label: 'Completed', icon: IconCircleCheck },
]

const secondaryNav: NavItem[] = [
  { to: '/settings', label: 'Settings', icon: IconSettings },
  { to: '/integrations', label: 'Integrations', icon: IconPlugConnected },
]

function isActive(pathname: string, to: string): boolean {
  return pathname === to || pathname.startsWith(`${to}/`)
}

export function App() {
  const navigate = useNavigate()
  const location = useLocation()
  const queryClient = useQueryClient()
  const [mobileOpened, { toggle: toggleMobile, close: closeMobile }] =
    useDisclosure(false)
  const [captureOpened, { open: openCapture, close: closeCapture }] =
    useDisclosure(false)
  const currentUser = useCurrentUser()
  const settings = useSettings()
  const { setColorScheme } = useMantineColorScheme()

  useEffect(() => {
    closeMobile()
  }, [location.pathname, closeMobile])

  // Apply the theme stored with the account when the app loads. The local
  // toggle stays instant; Settings persists the choice.
  useEffect(() => {
    if (settings.data) {
      setColorScheme(settings.data.theme)
    }
  }, [settings.data, setColorScheme])

  function handleLogout() {
    clearToken()
    queryClient.clear()
    navigate('/login', { replace: true })
  }

  const name = currentUser.data?.name ?? 'Your account'
  const email = currentUser.data?.email ?? ''

  return (
    <>
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>
      <AppShell
        header={{ height: 60 }}
        navbar={{
          width: 264,
          breakpoint: 'sm',
          collapsed: { mobile: !mobileOpened },
        }}
        padding="md"
      >
        <AppShell.Header>
          <Group h="100%" px="md" justify="space-between" wrap="nowrap">
            <Group gap="sm" wrap="nowrap">
              <Burger
                opened={mobileOpened}
                onClick={toggleMobile}
                hiddenFrom="sm"
                size="sm"
                aria-label="Toggle navigation"
              />
              <UnstyledButton component={Link} to="/dashboard">
                <Brand compact />
              </UnstyledButton>
            </Group>

            <Group gap="xs" wrap="nowrap">
              <Button
                visibleFrom="xs"
                leftSection={<IconPlus size={16} />}
                onClick={openCapture}
              >
                Quick capture
              </Button>
              <Tooltip label="Quick capture" withArrow>
                <ActionIcon
                  hiddenFrom="xs"
                  size="lg"
                  radius="md"
                  variant="filled"
                  aria-label="Quick capture"
                  onClick={openCapture}
                >
                  <IconPlus size={18} />
                </ActionIcon>
              </Tooltip>

              <ThemeToggle />

              <Menu position="bottom-end" withArrow>
                <Menu.Target>
                  <ActionIcon
                    size="lg"
                    radius="xl"
                    variant="light"
                    aria-label="Account menu"
                  >
                    <Avatar size="sm" radius="xl" color="indigo">
                      {initials(name)}
                    </Avatar>
                  </ActionIcon>
                </Menu.Target>
                <Menu.Dropdown>
                  <Menu.Label>
                    <Text size="sm" fw={600} lineClamp={1}>
                      {name}
                    </Text>
                    {email && (
                      <Text size="xs" c="dimmed" lineClamp={1}>
                        {email}
                      </Text>
                    )}
                  </Menu.Label>
                  <Menu.Divider />
                  <Menu.Item
                    leftSection={<IconUser size={16} />}
                    component={Link}
                    to="/settings"
                  >
                    Settings
                  </Menu.Item>
                  <Menu.Item
                    leftSection={<IconLogout size={16} />}
                    color="red"
                    onClick={handleLogout}
                  >
                    Log out
                  </Menu.Item>
                </Menu.Dropdown>
              </Menu>
            </Group>
          </Group>
        </AppShell.Header>

        <AppShell.Navbar p="sm">
          <AppShell.Section grow>
            <Stack gap={2}>
              {primaryNav.map((item) => (
                <NavLink
                  key={item.to}
                  component={RouterNavLink}
                  to={item.to}
                  label={item.label}
                  leftSection={<item.icon size={18} stroke={1.6} />}
                  active={isActive(location.pathname, item.to)}
                  variant="light"
                  style={{ borderRadius: 8 }}
                />
              ))}

              <Divider my="sm" />

              {secondaryNav.map((item) => (
                <NavLink
                  key={item.to}
                  component={RouterNavLink}
                  to={item.to}
                  label={item.label}
                  leftSection={<item.icon size={18} stroke={1.6} />}
                  active={isActive(location.pathname, item.to)}
                  variant="light"
                  style={{ borderRadius: 8 }}
                />
              ))}
            </Stack>
          </AppShell.Section>

          <AppShell.Section>
            <Box p="xs">
              <Text size="xs" c="dimmed">
                BrainDrop 0.1.0
              </Text>
              <Text size="xs" c="dimmed">
                Local-first. AI stays optional.
              </Text>
            </Box>
          </AppShell.Section>
        </AppShell.Navbar>

        <AppShell.Main id="main-content">
          <Outlet />
        </AppShell.Main>
      </AppShell>

      {captureOpened && (
        <Suspense fallback={null}>
          <QuickCapture
            opened
            onClose={closeCapture}
            defaults={{
              type: settings.data?.default_item_type,
              priority: settings.data?.default_priority,
            }}
            ai={settings.data?.ai}
          />
        </Suspense>
      )}
    </>
  )
}
