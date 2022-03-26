@echo off
set CLASSPATH=.;%~dp0
 
@
@REM Dynamic create the Jar classpath
@
for %%i in ("%~dp0"build\libs\*.jar) do @call :classpath-append "%%i"
for %%i in ("%~dp0"lib\*.jar) do @call :classpath-append "%%i"
goto :EOF
 
:classpath-append
set CLASSPATH=%CLASSPATH%;%~1
goto :EOF
:end 
