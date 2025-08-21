# Services often targeted
1. **SSH**: deposit of miners/scripts after login
2. **HTTP(S)**: Automated scans
3. **Telnet** (unencrypted SSH): Target #1 for IoT botnets (Mirai-like) with default passwords
4. **SMB**: Historical exploits (e.g., EternalBlue), NTLM theft/relay, Windows lateral movement
5. **FTP**: Anonymous connections/weak authentication, binary deposits


# Complexity exposure of selected services
We decided to honeypot SSH and HTTP(S) because these are currently the two most frequently attacked services. Here is a description of their exposure complexity
## SSH
#### 1. Exposure complexity
- Level: Basic
	- Create the `systemd-nspawn` container, install a vulnerable `sshd`, give it an IP (veth/bridge) and DNAT/publish a port on the host → it works quickly
	- Effort: low (a few commands)
- Level: Properly isolated
    - Dedicated network (isolated bridge or VLAN), DNAT from public port → container
	- Egress blocked (nftables/ip6tables: drop by default, allow-only to your log sink)
    - User namespaces (container root mapped to non-root UID on the host)
	- Out-of-the-box logging/export
	- Effort: medium
#### 2. Risks for the machine that exposes
- RCE / container takeover
	- Very likely if the vulnerability is exploitable
	- If egress is open: the container can attack the Internet, download malware, perform C2 → your IP will be blacklisted/abused
- Container escape (breakout)
	- Rare but possible via kernel vulnerabilities/capabilities/incorrect configurations
	- Incorrectly configured users or overly broad capabilities increase the risk
- Lateral movement
	- Without network segmentation, the attacker can scan/pivot to other internal systems
- Legal impact & reputation
	- Sending spam/DDoS from your IP → complaints, blocks, AUP sanctions
#### 3. Minimum guardrails
- Network: isolated bridge, incoming DNAT only, egress = DROP (except to your log collection)
- Accounts: no shared management access; keys and secrets do not exist/are fake
- Names/OS fingerprints: credible but fake
- nspawn: user namespaces enabled, FS in volatile/overlay mode, read-only, precise mounts (dedicated `/var/log`), minimal capabilities
- systemd (container unit): CPU/RAM quotas, `NoNewPrivileges=`, network AF restrictions, `DevicePolicy=closed`



## HTTP(S)
#### 1. Exposure complexity
- Level: Basic
	- `systemd-nspawn` container, installation of a web server (Apache/nginx) + vulnerable app (e.g., old PHP / deliberately vulnerable app)
	- IP via veth/bridge, DNAT of port 80/443 from host → container
    - Effort: low
- Level: Properly isolated
    - Dedicated network bridge, incoming DNAT only (no direct container exposure)
	- Egress blocked (DROP by default: no outgoing 53/80/443)
	- Read-only FS docroot
	- TLS on the container side (static cert), for credibility
    - Effort: medium
#### 2. Risks for the machine that exposes
- RCE / webshell
	- Classic vulnerabilities (unfiltered upload, LFI/RFI, deserialization, injections) → take controlle of the container
	- With open egress: tool downloads, C2, outgoing scans
- Open proxy / SSRF / pivot
	- Configuration mismatch (mod_proxy/CONNECT, redirects) or SSRF endpoints → the attacker uses your IP address to access the Internet or your internal network (cloud metadata, etc.)
- Illegal hosting/file dropping
	- Uploads visible from the web → distribution of malware/illegal content → blacklists/complaints
- Application DoS
	- Huge bodies, slowloris/slow POST → exhaustion of container CPU/RAM (or even host CPU/RAM without cgroups)
#### 3. Minimum guardrails
- Network
	- Isolated bridge + incoming DNAT only
	- Egress = DROP by default
	- No forwarding to other internal subnets
- HTTP(S)
	- Disable mod_proxy, CONNECT, proxy_pass, and any outgoing reverse proxies
	- No autoindex, no direct links to the upload area
	- Uploads: store outside docroot, noexec, nosuid, nodev, hash + quarantined, never served publicly
	- TLS: static cert (no auto ACME if egress blocked)
- nspawn/systemd
	- User namespaces (root container mapped to non-privileged UID on the host)
	- FS overlay/volatile, read-only root; strictly listed write directories
