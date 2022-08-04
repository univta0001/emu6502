@echo off
java --module-path C:\apps\javafx-sdk-18.0.2\lib --add-modules javafx.controls,javafx.fxml,javafx.graphics,javafx.base,javafx.media --add-opens javafx.graphics/com.sun.glass.ui=ALL-UNNAMED -jar jace.jar
