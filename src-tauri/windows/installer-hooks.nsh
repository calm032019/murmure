; Built-in AI model: shipped next to Murmure_x64-setup.exe because NSIS
; installers cannot exceed ~2 GB. If the model sits beside the installer,
; copy it in; otherwise Murmure offers a one-time download in the app.
!define MURMURE_LLM_MODEL "Qwen_Qwen3.5-4B-Q4_K_M.gguf"

!macro NSIS_HOOK_POSTINSTALL
  ${If} ${FileExists} "$EXEDIR\${MURMURE_LLM_MODEL}"
  ${AndIfNot} ${FileExists} "$INSTDIR\models\${MURMURE_LLM_MODEL}"
    DetailPrint "Installing the built-in AI model (about 3 GB)..."
    CreateDirectory "$INSTDIR\models"
    CopyFiles /SILENT "$EXEDIR\${MURMURE_LLM_MODEL}" "$INSTDIR\models"
  ${EndIf}
!macroend

; Keep the model across updates; remove it on a real uninstall.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    Delete "$INSTDIR\models\${MURMURE_LLM_MODEL}"
    RMDir "$INSTDIR\models"
  ${EndIf}
!macroend
