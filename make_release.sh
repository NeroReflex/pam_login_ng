new_version=${1}
version=$(grep -oP 'version\s*=\s*"\K[0-9]+\.[0-9]+\.[0-9]+' login_ng/Cargo.toml)
find . -name "*.toml"  -type f -exec sed -i "s/$version/$new_version/g" {} +
git tag $new_version