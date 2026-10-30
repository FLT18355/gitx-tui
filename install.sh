#!/bin/sh
# gxt 本地安装脚本喵~
# 在仓库根目录执行：./install.sh
# 它会 cargo build --release，然后把二进制装到 ~/.local/bin 喵~
set -eu

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN="gxt"

usage() {
    cat <<'EOF'
gxt 本地安装脚本喵~

用法：
  ./install.sh [--dir <安装目录>] [--debug] [--no-locked]

选项：
  --dir <目录>   安装目录，默认 ~/.local/bin 喵~
  --debug        构建 debug 版本（更快，但比较大）喵~
  --no-locked    不传 --locked 给 cargo 喵~
  -h, --help     看这段帮助喵~

环境变量：
  INSTALL_DIR    等同于 --dir
EOF
}

die() {
    printf '安装失败喵~：%s\n' "$*" >&2
    exit 1
}

have() {
    command -v "$1" >/dev/null 2>&1
}

profile="release"
locked="--locked"

while [ $# -gt 0 ]; do
    case "$1" in
        --dir)
            [ $# -ge 2 ] || die "--dir 后面要写目录喵~"
            INSTALL_DIR="$2"
            shift 2
            ;;
        --debug)
            profile="debug"
            shift
            ;;
        --no-locked)
            locked=""
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "不认识的参数：$1 喵~"
            ;;
    esac
done

[ -n "$INSTALL_DIR" ] || die "安装目录不能是空的喵~"

# 允许从仓库内任意位置执行，也允许 `sh path/to/install.sh`
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd || pwd)"
cd "$script_dir" || die "进不去脚本所在目录喵~"
[ -f Cargo.toml ] || die "这里没有 Cargo.toml，请在 gxt 仓库根目录运行喵~"

have cargo || die "没有找到 cargo，先安装 Rust（https://rustup.rs）喵~"

printf '人家开始 cargo build --%s 啦喵~...\n' "$profile"
if [ "$profile" = "release" ]; then
    # shellcheck disable=SC2086
    cargo build --release $locked || die "编译没有成功喵~，往上翻翻报错"
    built="target/release/$BIN"
else
    # shellcheck disable=SC2086
    cargo build $locked || die "编译没有成功喵~，往上翻翻报错"
    built="target/debug/$BIN"
fi

[ -f "$built" ] || die "编译完了但没找到 $built 喵~"

mkdir -p "$INSTALL_DIR" || die "安装目录建不出来：$INSTALL_DIR 喵~"
install -m 755 "$built" "$INSTALL_DIR/$BIN" 2>/dev/null ||
    { cp "$built" "$INSTALL_DIR/$BIN" && chmod 755 "$INSTALL_DIR/$BIN"; } ||
    die "没有权限写到 $INSTALL_DIR 喵~，换 --dir 吧喵~"

printf '装好啦喵~：%s/%s\n' "$INSTALL_DIR" "$BIN"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) printf '提示：把 %s 加到 PATH 里就能直接敲 %s 啦喵~\n' "$INSTALL_DIR" "$BIN" ;;
esac
