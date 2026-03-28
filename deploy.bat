@echo off
REM hyper-static-server Docker 部署脚本 (Windows)
REM 用法: deploy.bat [dev|prod|build|stop]

setlocal enabledelayedexpansion

set PROJECT_NAME=hyper-static-server
set COMPOSE_FILE_DEV=docker-compose.dev.yml
set COMPOSE_FILE_PROD=docker-compose.yml
set COMPOSE_FILE_PROD_FULL=docker-compose.prod.yml

REM 颜色输出 (Windows CMD 不支持 ANSI 颜色，这里使用简单文本)
set INFO=[INFO]
set SUCCESS=[SUCCESS]
set WARN=[WARN]
set ERROR=[ERROR]

:log_info
echo %INFO% %~1
goto :eof

:log_success
echo %SUCCESS% %~1
goto :eof

:log_warn
echo %WARN% %~1
goto :eof

:log_error
echo %ERROR% %~1
goto :eof

REM 检查 Docker 是否安装
:check_docker
where docker >nul 2>nul
if %errorlevel% neq 0 (
    call :log_error "Docker 未安装。请先安装 Docker。"
    exit /b 1
)

where docker-compose >nul 2>nul
if %errorlevel% neq 0 (
    call :log_error "Docker Compose 未安装。请先安装 Docker Compose。"
    exit /b 1
)
goto :eof

REM 构建镜像
:build_image
call :log_info "构建 %PROJECT_NAME% 镜像..."
docker build -t %PROJECT_NAME%:latest .
call :log_success "镜像构建完成"
goto :eof

REM 开发环境部署
:deploy_dev
call :log_info "启动开发环境..."
docker-compose -f %COMPOSE_FILE_DEV% up --build -d
call :log_success "开发环境已启动"
call :log_info "访问地址: http://localhost:3000"
call :log_info "查看日志: docker-compose -f %COMPOSE_FILE_DEV% logs -f"
goto :eof

REM 生产环境部署
:deploy_prod
call :log_info "启动生产环境..."
docker-compose -f %COMPOSE_FILE_PROD_FULL% up --build -d
call :log_success "生产环境已启动"
call :log_info "访问地址: http://localhost"
call :log_info "查看日志: docker-compose -f %COMPOSE_FILE_PROD_FULL% logs -f"
goto :eof

REM 停止服务
:stop_services
call :log_info "停止所有服务..."

REM 尝试停止开发环境
docker-compose -f %COMPOSE_FILE_DEV% ps | findstr "Up" >nul
if %errorlevel% equ 0 (
    docker-compose -f %COMPOSE_FILE_DEV% down
    call :log_success "开发环境已停止"
)

REM 尝试停止生产环境
docker-compose -f %COMPOSE_FILE_PROD_FULL% ps | findstr "Up" >nul
if %errorlevel% equ 0 (
    docker-compose -f %COMPOSE_FILE_PROD_FULL% down
    call :log_success "生产环境已停止"
)

REM 停止默认环境
docker-compose -f %COMPOSE_FILE_PROD% ps | findstr "Up" >nul
if %errorlevel% equ 0 (
    docker-compose -f %COMPOSE_FILE_PROD% down
    call :log_success "默认环境已停止"
)
goto :eof

REM 显示状态
:show_status
call :log_info "服务状态:"

echo 开发环境:
docker-compose -f %COMPOSE_FILE_DEV% ps

echo.
echo 生产环境:
docker-compose -f %COMPOSE_FILE_PROD_FULL% ps

echo.
echo 默认环境:
docker-compose -f %COMPOSE_FILE_PROD% ps
goto :eof

REM 显示帮助
:show_help
echo hyper-static-server Docker 部署脚本 (Windows)
echo.
echo 用法: %0 [命令]
echo.
echo 命令:
echo   dev     启动开发环境 (带热重载)
echo   prod    启动生产环境 (带 Nginx 反向代理)
echo   build   仅构建 Docker 镜像
echo   stop    停止所有服务
echo   status  显示服务状态
echo   logs    查看服务日志
echo   help    显示此帮助信息
echo.
echo 示例:
echo   %0 dev     # 启动开发环境
echo   %0 prod    # 启动生产环境
echo   %0 stop    # 停止所有服务
goto :eof

REM 查看日志
:show_logs
echo 选择要查看的环境日志:
echo 1) 开发环境
echo 2) 生产环境
echo 3) 默认环境
set /p choice="请选择 (1-3): "

if "%choice%"=="1" (
    docker-compose -f %COMPOSE_FILE_DEV% logs -f
) else if "%choice%"=="2" (
    docker-compose -f %COMPOSE_FILE_PROD_FULL% logs -f
) else if "%choice%"=="3" (
    docker-compose -f %COMPOSE_FILE_PROD% logs -f
) else (
    call :log_error "无效选择"
    exit /b 1
)
goto :eof

REM 主函数
:main
call :check_docker

if "%1"=="" goto show_help
if "%1"=="dev" goto deploy_dev
if "%1"=="prod" goto deploy_prod
if "%1"=="build" goto build_image
if "%1"=="stop" goto stop_services
if "%1"=="status" goto show_status
if "%1"=="logs" goto show_logs
if "%1"=="help" goto show_help

REM 默认显示帮助
goto show_help