#!/bin/bash

# hyper-static-server Docker 部署脚本
# 用法: ./deploy.sh [dev|prod|build|stop]

set -e

PROJECT_NAME="hyper-static-server"
COMPOSE_FILE_DEV="docker-compose.dev.yml"
COMPOSE_FILE_PROD="docker-compose.yml"
COMPOSE_FILE_PROD_FULL="docker-compose.prod.yml"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查 Docker 是否安装
check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker 未安装。请先安装 Docker。"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose 未安装。请先安装 Docker Compose。"
        exit 1
    fi
}

# 构建镜像
build_image() {
    log_info "构建 $PROJECT_NAME 镜像..."
    docker build -t $PROJECT_NAME:latest .
    log_success "镜像构建完成"
}

# 开发环境部署
deploy_dev() {
    log_info "启动开发环境..."
    docker-compose -f $COMPOSE_FILE_DEV up --build -d
    log_success "开发环境已启动"
    log_info "访问地址: http://localhost:3000"
    log_info "查看日志: docker-compose -f $COMPOSE_FILE_DEV logs -f"
}

# 生产环境部署
deploy_prod() {
    log_info "启动生产环境..."
    docker-compose -f $COMPOSE_FILE_PROD_FULL up --build -d
    log_success "生产环境已启动"
    log_info "访问地址: http://localhost"
    log_info "查看日志: docker-compose -f $COMPOSE_FILE_PROD_FULL logs -f"
}

# 停止服务
stop_services() {
    log_info "停止所有服务..."

    # 尝试停止开发环境
    if docker-compose -f $COMPOSE_FILE_DEV ps | grep -q "Up"; then
        docker-compose -f $COMPOSE_FILE_DEV down
        log_success "开发环境已停止"
    fi

    # 尝试停止生产环境
    if docker-compose -f $COMPOSE_FILE_PROD_FULL ps | grep -q "Up"; then
        docker-compose -f $COMPOSE_FILE_PROD_FULL down
        log_success "生产环境已停止"
    fi

    # 停止默认环境
    if docker-compose -f $COMPOSE_FILE_PROD ps | grep -q "Up"; then
        docker-compose -f $COMPOSE_FILE_PROD down
        log_success "默认环境已停止"
    fi
}

# 显示状态
show_status() {
    log_info "服务状态:"

    echo "开发环境:"
    docker-compose -f $COMPOSE_FILE_DEV ps

    echo -e "\n生产环境:"
    docker-compose -f $COMPOSE_FILE_PROD_FULL ps

    echo -e "\n默认环境:"
    docker-compose -f $COMPOSE_FILE_PROD ps
}

# 显示帮助
show_help() {
    echo "hyper-static-server Docker 部署脚本"
    echo ""
    echo "用法: $0 [命令]"
    echo ""
    echo "命令:"
    echo "  dev     启动开发环境 (带热重载)"
    echo "  prod    启动生产环境 (带 Nginx 反向代理)"
    echo "  build   仅构建 Docker 镜像"
    echo "  stop    停止所有服务"
    echo "  status  显示服务状态"
    echo "  logs    查看服务日志"
    echo "  help    显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0 dev     # 启动开发环境"
    echo "  $0 prod    # 启动生产环境"
    echo "  $0 stop    # 停止所有服务"
}

# 查看日志
show_logs() {
    echo "选择要查看的环境日志:"
    echo "1) 开发环境"
    echo "2) 生产环境"
    echo "3) 默认环境"
    read -p "请选择 (1-3): " choice

    case $choice in
        1)
            docker-compose -f $COMPOSE_FILE_DEV logs -f
            ;;
        2)
            docker-compose -f $COMPOSE_FILE_PROD_FULL logs -f
            ;;
        3)
            docker-compose -f $COMPOSE_FILE_PROD logs -f
            ;;
        *)
            log_error "无效选择"
            exit 1
            ;;
    esac
}

# 主函数
main() {
    check_docker

    case "${1:-help}" in
        "dev")
            deploy_dev
            ;;
        "prod")
            deploy_prod
            ;;
        "build")
            build_image
            ;;
        "stop")
            stop_services
            ;;
        "status")
            show_status
            ;;
        "logs")
            show_logs
            ;;
        "help"|*)
            show_help
            ;;
    esac
}

main "$@"