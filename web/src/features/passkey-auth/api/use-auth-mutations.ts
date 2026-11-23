import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { describeError } from "@/shared/api";
import { toast } from "@/shared/ui/toaster";

import { recover, signIn, signInWithAutofill, signOut, signUp } from "./ceremonies";

export function useSignUp() {
  return useMutation({
    mutationFn: signUp,
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useSignIn() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: signIn,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["me"] }),
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useAutofillSignIn() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: signInWithAutofill,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["me"] }),
  });
}

export function useRecover() {
  return useMutation({
    mutationFn: recover,
    onError: (error) => toast.error(describeError(error)),
  });
}

export function useSignOut() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  return useMutation({
    mutationFn: signOut,
    onSuccess: async () => {
      await navigate({ to: "/login" });
      queryClient.clear();
    },
    onError: (error) => toast.error(describeError(error)),
  });
}
