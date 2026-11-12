import { createFormHook } from "@tanstack/react-form";
import { fieldContext, formContext } from "./context";
import { FormInput, FormSelect, FormTextArea } from "./fields";

export type { SelectOption } from "./fields";

/** The one form hook of the app: fields are HeroUI controls bound to TanStack Form. */
export const { useAppForm, withForm } = createFormHook({
	fieldComponents: { FormInput, FormTextArea, FormSelect },
	formComponents: {},
	fieldContext,
	formContext,
});
