import { Button, Card } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { ChevronRight, Database, LockKeyhole, Send, ShieldCheck } from "lucide-react";
import { z } from "zod";
import { devLogin, getCsrfToken, getDevLoginStatus, login } from "#/api/endpoints/auth/auth";
import type { MeResponse } from "#/api/models";
import { authKeys } from "#/api/query-keys";
import { ApiError } from "#/components/ui";
import { useAppForm } from "#/forms";
import { type I18n, useI18n } from "#/i18n";
import { LanguageSwitch } from "#/i18n/LanguageSwitch";

const route = getRouteApi("/login");

const schema = z.object({
	login: z.string().trim().min(1, "Enter your login"),
	password: z.string().min(1, "Enter your password"),
});

const DEMO_LOGINS: { login: string; role: (t: I18n["t"]) => string }[] = [
	{
		login: "perf.manager.0001",
		role: (t) => t("Manager - wszyscy klienci i projekty", "Manager - every client and project"),
	},
	{
		login: "perf.sales.0001",
		role: (t) =>
			t("Sprzedawca - swoi klienci, projekty zespołu", "Sales - own clients, team projects"),
	},
	{
		login: "perf.admin.0001",
		role: (t) => t("Admin - użytkownicy i zespoły", "Admin - users and teams"),
	},
	{
		login: "anna.nowak",
		role: (t) => t("Agentka service desku - pula zgłoszeń", "Service desk agent - the ticket pool"),
	},
];

const FACTS = [
	{
		icon: Database,
		text: "5 000 000 clients and projects, every call one PostgreSQL procedure or function",
	},
	{ icon: ShieldCheck, text: "The app login may only use schema app" },
	{ icon: LockKeyhole, text: "Rust session, CSRF cookie, lockout after failed logins" },
];

export function LoginPage() {
	const { t } = useI18n();
	const search = route.useSearch();
	const navigate = useNavigate();
	const queryClient = useQueryClient();

	const signedIn = (me: MeResponse) => {
		queryClient.setQueryData(authKeys.me(), me);
		void navigate({ to: search.redirect ?? "/" });
	};

	const mutation = useMutation({
		mutationFn: async (value: z.infer<typeof schema>) => {
			// The first write needs the XSRF-TOKEN cookie; this request sets it.
			await getCsrfToken();
			return login({ login: value.login, password: value.password });
		},
		onSuccess: signedIn,
	});

	// One-click login without a password, only when the server runs with CRM_DEV_LOGIN=true (local stack).
	const devLoginStatus = useQuery({
		queryKey: authKeys.devLogin(),
		queryFn: () => getDevLoginStatus(),
		staleTime: Number.POSITIVE_INFINITY,
	});
	const quickLogin = useMutation({
		mutationFn: async (userLogin: string) => {
			await getCsrfToken();
			return devLogin({ login: userLogin });
		},
		onSuccess: signedIn,
	});
	const devLoginEnabled = devLoginStatus.data?.enabled === true;

	const form = useAppForm({
		defaultValues: { login: "", password: "" },
		validators: { onChange: schema },
		onSubmit: ({ value }) => mutation.mutate(value),
	});

	return (
		<div className="grid min-h-screen lg:grid-cols-[1.05fr_1fr]">
			<section className="relative hidden overflow-hidden bg-sidebar px-14 py-12 text-sidebar-text lg:flex lg:flex-col">
				<div
					aria-hidden
					className="pointer-events-none absolute -top-40 -right-40 size-[520px] rounded-full border border-champagne/20"
				/>
				<div
					aria-hidden
					className="pointer-events-none absolute -top-24 -right-24 size-[360px] rounded-full border border-champagne/10"
				/>
				<div className="flex items-center gap-3">
					<img src="/favicon.svg" alt="" className="size-9" />
					<span className="font-mono text-[11px] tracking-[0.16em] text-sidebar-muted uppercase">
						R CRM / Rust
					</span>
				</div>

				<div className="mt-auto max-w-lg">
					<p className="font-mono text-[11px] tracking-[0.16em] text-champagne uppercase">
						Clients · Projects · Teams
					</p>
					<h1 className="mt-4 font-display text-6xl leading-[1.05] font-semibold text-white">
						Every client,
						<br />
						every project,
						<br />
						<span className="text-champagne">one procedure away.</span>
					</h1>
					<ul className="mt-10 flex flex-col gap-4">
						{FACTS.map((fact) => (
							<li key={fact.text} className="flex items-start gap-3 text-sm">
								<fact.icon className="mt-0.5 size-4 shrink-0 text-champagne" strokeWidth={1.75} />
								<span>{fact.text}</span>
							</li>
						))}
					</ul>
				</div>

				<p className="mt-12 text-xs text-sidebar-muted">
					Rust · axum · PostgreSQL 18 · React · HeroUI
				</p>
			</section>

			<section className="flex items-center justify-center px-6 py-12">
				<div className="w-full max-w-sm">
					<div className="mb-8 lg:hidden">
						<img src="/favicon.svg" alt="" className="size-10" />
					</div>
					<div className="mb-6 flex items-center justify-between gap-3">
						<LanguageSwitch />
						<Link
							to="/report"
							className="inline-flex items-center gap-1 text-sm text-muted hover:text-foreground"
						>
							<Send className="size-4" />
							{t("Zgłoś sprawę bez logowania", "Report a request without signing in")}
						</Link>
					</div>
					<h2 className="font-display text-4xl font-semibold">{t("Logowanie", "Sign in")}</h2>
					<p className="mt-2 text-sm text-muted">
						{t("Login i hasło do CRM.", "Use your CRM login and password.")}
					</p>

					<form
						noValidate
						className="mt-8 flex flex-col gap-5"
						onSubmit={(event) => {
							event.preventDefault();
							void form.handleSubmit();
						}}
					>
						<form.AppField name="login">
							{(field) => (
								<field.FormInput
									label={t("Login", "Login")}
									autoComplete="username"
									placeholder="name.surname"
									autoFocus
								/>
							)}
						</form.AppField>
						<form.AppField name="password">
							{(field) => (
								<field.FormInput
									label={t("Hasło", "Password")}
									type="password"
									autoComplete="current-password"
								/>
							)}
						</form.AppField>
						<ApiError
							error={mutation.error ?? quickLogin.error}
							title={t("Logowanie nie powiodło się", "Sign-in failed")}
						/>
						<Button type="submit" fullWidth size="lg" isPending={mutation.isPending}>
							{t("Zaloguj", "Sign in")}
						</Button>
					</form>

					<Card className="mt-10" variant="secondary">
						<Card.Header>
							<Card.Title className="font-mono text-[11px] tracking-[0.12em] uppercase">
								{t("Konta z seeda", "Seeded accounts")}
							</Card.Title>
							<Card.Description>
								{devLoginEnabled
									? t(
											"› loguje od razu (CRM_DEV_LOGIN); albo wybierz login i wpisz CRM_SEED_PASSWORD.",
											"Press › to sign in at once (CRM_DEV_LOGIN), or pick a login and type CRM_SEED_PASSWORD.",
										)
									: t(
											"Hasło: CRM_SEED_PASSWORD z infrastructure/.env",
											"Password: CRM_SEED_PASSWORD from infrastructure/.env",
										)}
							</Card.Description>
						</Card.Header>
						<Card.Content className="flex flex-col gap-1">
							{DEMO_LOGINS.map((demo) => (
								<div key={demo.login} className="flex items-center gap-1">
									<button
										type="button"
										className="flex min-w-0 flex-1 items-baseline justify-between gap-3 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-default"
										onClick={() => form.setFieldValue("login", demo.login)}
									>
										<span className="font-mono text-xs text-foreground">{demo.login}</span>
										<span className="text-[11px] text-muted">{demo.role(t)}</span>
									</button>
									{devLoginEnabled && (
										<Button
											isIconOnly
											size="sm"
											variant="ghost"
											aria-label={t(`Zaloguj jako ${demo.login}`, `Sign in as ${demo.login}`)}
											isPending={quickLogin.isPending && quickLogin.variables === demo.login}
											isDisabled={quickLogin.isPending}
											onPress={() => quickLogin.mutate(demo.login)}
										>
											<ChevronRight className="size-4" />
										</Button>
									)}
								</div>
							))}
						</Card.Content>
					</Card>
				</div>
			</section>
		</div>
	);
}
