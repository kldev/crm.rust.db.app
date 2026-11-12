import { Button, Chip, Drawer, Dropdown, Label, Separator, toast } from "@heroui/react";
import { useMutation } from "@tanstack/react-query";
import { Link, Outlet, useMatches, useNavigate } from "@tanstack/react-router";
import {
	BriefcaseBusiness,
	Building2,
	ChartColumn,
	Contact,
	FileSpreadsheet,
	Headset,
	Inbox,
	KeyRound,
	LayoutDashboard,
	ListTodo,
	LogOut,
	Menu,
	SquarePlus,
	Tickets,
	UserCog,
	UsersRound,
} from "lucide-react";
import { type ReactNode, useRef, useState } from "react";
import { http } from "#/api/mutator";
import { UserAvatar } from "#/components/ui";
import {
	type ChangePasswordCommand,
	ChangePasswordDrawer,
} from "#/features/auth/ChangePasswordDrawer";
import { can, type Role, useForgetSession, useMe } from "#/features/auth/session";
import { DeskCounters } from "#/features/desk/components/DeskCounters";
import { roleLabel } from "#/features/types";
import { type I18n, useI18n } from "#/i18n";
import { LanguageSwitch } from "#/i18n/LanguageSwitch";

interface NavItem {
	to:
		| "/"
		| "/clients"
		| "/projects"
		| "/teams"
		| "/users"
		| "/desk/pool"
		| "/desk/mine"
		| "/desk/all"
		| "/desk/new"
		| "/desk/agents"
		| "/desk/people"
		| "/desk/reports"
		| "/desk/charts";
	label: (t: I18n["t"]) => string;
	icon: typeof LayoutDashboard;
	visible?: (role: Role) => boolean;
}

const NAV: { section: (t: I18n["t"]) => string; items: NavItem[] }[] = [
	{
		section: (t) => t("CRM", "CRM"),
		items: [
			{ to: "/", label: (t) => t("Przegląd", "Overview"), icon: LayoutDashboard },
			{ to: "/clients", label: (t) => t("Klienci", "Clients"), icon: Building2 },
			{ to: "/projects", label: (t) => t("Projekty", "Projects"), icon: BriefcaseBusiness },
			{ to: "/teams", label: (t) => t("Zespoły", "Teams"), icon: UsersRound },
			{
				to: "/users",
				label: (t) => t("Użytkownicy", "Users"),
				icon: UserCog,
				visible: can.seeUsers,
			},
		],
	},
	{
		section: (t) => t("Service desk", "Service desk"),
		items: [
			{ to: "/desk/pool", label: (t) => t("Pula", "Pool"), icon: Inbox },
			{ to: "/desk/mine", label: (t) => t("Moje zgłoszenia", "My tickets"), icon: ListTodo },
			{ to: "/desk/all", label: (t) => t("Wszystkie", "All tickets"), icon: Tickets },
			{ to: "/desk/new", label: (t) => t("Nowe zgłoszenie", "New ticket"), icon: SquarePlus },
			{ to: "/desk/agents", label: (t) => t("Agenci", "Agents"), icon: Headset },
			{ to: "/desk/people", label: (t) => t("Osoby", "People"), icon: Contact },
			{ to: "/desk/reports", label: (t) => t("Raporty", "Reports"), icon: FileSpreadsheet },
			{ to: "/desk/charts", label: (t) => t("Wykresy", "Charts"), icon: ChartColumn },
		],
	},
];

function Brand() {
	return (
		<div className="flex items-center gap-3 px-5 pt-6 pb-8">
			<img src="/favicon.svg" alt="" className="size-8" />
			<div className="leading-tight">
				<p className="font-display text-2xl font-semibold text-white">R CRM</p>
				<p className="font-mono text-[10px] tracking-[0.14em] text-sidebar-muted uppercase">Rust</p>
			</div>
		</div>
	);
}

function Navigation({ role, onNavigate }: { role: Role; onNavigate?: () => void }) {
	const { t } = useI18n();
	return (
		<nav className="flex flex-col gap-1 px-3" aria-label={t("Główna", "Main")}>
			{NAV.map((group) => (
				<div key={group.section(t)} className="flex flex-col gap-1 pb-4">
					<p className="px-3 pb-2 font-mono text-[10px] tracking-[0.14em] text-sidebar-muted uppercase">
						{group.section(t)}
					</p>
					<NavLinks items={group.items} role={role} onNavigate={onNavigate} />
				</div>
			))}
		</nav>
	);
}

function NavLinks({
	items,
	role,
	onNavigate,
}: {
	items: NavItem[];
	role: Role;
	onNavigate?: () => void;
}) {
	const { t } = useI18n();
	return (
		<>
			{items
				.filter((item) => !item.visible || item.visible(role))
				.map((item) => (
					<Link
						key={item.to}
						to={item.to}
						onClick={onNavigate}
						activeOptions={{ exact: item.to === "/" }}
						className="group relative flex items-center gap-3 rounded-md px-3 py-2 text-sm text-sidebar-text transition-colors hover:bg-sidebar-hover data-[status=active]:bg-sidebar-active data-[status=active]:text-champagne"
					>
						<span className="absolute inset-y-1 left-0 w-0.5 rounded-full bg-champagne opacity-0 transition-opacity group-data-[status=active]:opacity-100" />
						<item.icon className="size-4" strokeWidth={1.75} />
						{item.label(t)}
					</Link>
				))}
		</>
	);
}

function UserMenu({ onChangePassword }: { onChangePassword: () => void }) {
	const { t } = useI18n();
	const me = useMe();
	const navigate = useNavigate();
	const forget = useForgetSession();
	const logout = useMutation({
		mutationFn: () => http.post("/api/auth/logout"),
		onSettled: () => {
			forget();
			void navigate({ to: "/login" });
		},
	});

	return (
		<Dropdown>
			<Dropdown.Trigger>
				<Button
					variant="ghost"
					className="h-auto gap-3 px-2 py-1.5"
					aria-label={t("Konto", "Account")}
				>
					<UserAvatar name={me.displayName} />
					<span className="hidden text-left leading-tight sm:block">
						<span className="block text-sm font-medium text-foreground">{me.displayName}</span>
						<span className="block font-mono text-[11px] text-muted">{me.login}</span>
					</span>
				</Button>
			</Dropdown.Trigger>
			<Dropdown.Popover placement="bottom end">
				<Dropdown.Menu
					onAction={(key) => {
						if (key === "password") onChangePassword();
						if (key === "logout") logout.mutate();
					}}
				>
					<Dropdown.Item id="role" isDisabled textValue={t("Rola", "Role")}>
						<Label>{t("Zalogowano jako", "Signed in as")}</Label>
						<Chip size="sm" variant="soft">
							{roleLabel[me.role]}
						</Chip>
					</Dropdown.Item>
					<Separator />
					<Dropdown.Item id="password" textValue={t("Zmień hasło", "Change password")}>
						<KeyRound className="size-4" />
						<Label>{t("Zmień hasło", "Change password")}</Label>
					</Dropdown.Item>
					<Dropdown.Item id="logout" textValue={t("Wyloguj", "Log out")} variant="danger">
						<LogOut className="size-4" />
						<Label>{t("Wyloguj", "Log out")}</Label>
					</Dropdown.Item>
				</Dropdown.Menu>
			</Dropdown.Popover>
		</Dropdown>
	);
}

export function AppLayout() {
	const { t } = useI18n();
	// Wide pages (reports with many columns, charts) use the whole screen; the rest keep a readable width.
	const wide = useMatches({ select: (matches) => matches.some((m) => m.staticData?.wide) });
	const me = useMe();
	const [mobileNav, setMobileNav] = useState(false);
	const passwordDrawer = useRef<ChangePasswordCommand>(null);

	return (
		<div className="min-h-screen bg-background lg:grid lg:grid-cols-[248px_minmax(0,1fr)]">
			<aside className="sticky top-0 hidden h-screen flex-col border-r border-sidebar-border bg-sidebar lg:flex">
				<Brand />
				<Navigation role={me.role} />
				<p className="mt-auto px-6 pb-6 text-[11px] leading-relaxed text-sidebar-muted">
					{t(
						"Każdy odczyt i zapis to procedura lub funkcja PostgreSQL.",
						"Every read and write is a PostgreSQL procedure or function.",
					)}
				</p>
			</aside>

			<Drawer>
				<Drawer.Backdrop isOpen={mobileNav} onOpenChange={setMobileNav}>
					<Drawer.Content placement="left" className="max-w-[270px]">
						<Drawer.Dialog className="bg-sidebar">
							<Brand />
							<Navigation role={me.role} onNavigate={() => setMobileNav(false)} />
						</Drawer.Dialog>
					</Drawer.Content>
				</Drawer.Backdrop>
			</Drawer>

			<div className="flex min-w-0 flex-col">
				<header className="sticky top-0 z-20 flex h-16 items-center gap-3 border-b border-separator bg-topbar/95 px-4 backdrop-blur sm:px-8">
					<Button
						isIconOnly
						variant="ghost"
						className="lg:hidden"
						aria-label={t("Otwórz nawigację", "Open navigation")}
						onPress={() => setMobileNav(true)}
					>
						<Menu className="size-5" />
					</Button>
					<DeskCounters />
					<div className="ml-auto flex items-center gap-3">
						<LanguageSwitch />
						<UserMenu onChangePassword={() => passwordDrawer.current?.open()} />
					</div>
				</header>
				<main
					className={`mx-auto w-full flex-1 px-4 py-8 sm:px-8 ${wide ? "max-w-none" : "max-w-7xl"}`}
				>
					<Outlet />
				</main>
			</div>

			<ChangePasswordDrawer
				ref={passwordDrawer}
				onSuccess={() => toast.success(t("Hasło zmienione", "Password changed"))}
			/>
		</div>
	);
}

/** Page title in the Obsidian serif, optional eyebrow, description and actions. */
export function PageHeader({
	eyebrow,
	title,
	description,
	actions,
	meta,
}: {
	eyebrow?: ReactNode;
	title: ReactNode;
	description?: ReactNode;
	actions?: ReactNode;
	meta?: ReactNode;
}) {
	return (
		<div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
			<div className="min-w-0">
				{eyebrow ? (
					<div className="mb-2 font-mono text-[11px] tracking-[0.12em] text-muted uppercase">
						{eyebrow}
					</div>
				) : null}
				<h1 className="font-display text-4xl leading-tight font-semibold text-foreground">
					{title}
				</h1>
				{description ? <p className="mt-2 max-w-2xl text-sm text-muted">{description}</p> : null}
				{meta ? <div className="mt-3 flex flex-wrap items-center gap-2">{meta}</div> : null}
			</div>
			{actions ? <div className="flex shrink-0 flex-wrap gap-2">{actions}</div> : null}
		</div>
	);
}
