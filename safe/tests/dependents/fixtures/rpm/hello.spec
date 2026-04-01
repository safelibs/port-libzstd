Name: hello
Version: 1.0
Release: 1
Summary: hello fixture for rpm zstd payload coverage
License: MIT
BuildArch: noarch
Source0: hello.txt

%description
Checked-in fixture for validating rpm zstd payload handling.

%prep

%build

%install
install -Dm0644 %{SOURCE0} %{buildroot}%{_datadir}/hello-rpm/hello.txt

%files
%{_datadir}/hello-rpm/hello.txt
