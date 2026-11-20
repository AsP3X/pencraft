# Non-app design formats: specifications and craft rules (marketing pages, social and ad graphics, slides, print, email, store assets) as of November 2026

Reading notes for the report writer:

- Research date: 20 November 2026. Each finding names its source; where the source page carried a date it is given.
- "Fetched" means the page itself was read. "Search summary" means the fact came from a search-result summary of that page and the page was not opened; treat those as second-hand.
- Pixel values at 300 dpi that are marked "computed" were calculated by me from the cited millimetre or inch size (px = mm / 25.4 x 300). They are arithmetic, not sourced figures.
- Pages that could not be read: Tailwind Plus block pages (login wall), guykawasaki.com (HTTP 403), duarte.com glance-test post (404), moo.com design guidelines (404), YC Startup Library pages (title only), Envato Elements counts (not shown on page). The affected items are listed under Gaps.

## 1. Landing pages and marketing sites: frame widths, rhythm, section inventory, pricing conventions, research rules, designed versus generated

### Takeaway
The only hard numbers that could be verified are the Tailwind width scale (1280 px content container, 1152 px one step down, breakpoints at 640 / 768 / 1024 / 1280 / 1536) and typographic ranges (45 to 75 characters per line, hero line height 1.0 to 1.1, section padding around 80 to 128 px); the measured anatomy of Tailwind Plus, shadcn and Relume blocks sits behind logins and was not verified. The 2026 critiques of generated pages agree closely with each other and with Pencraft's existing list of tells, and add two that are about content rather than looks: the stock section order and the mad-lib headline.

### Cited Findings

Widths and breakpoints
- Tailwind CSS default breakpoints: sm 640 px, md 768 px, lg 1024 px, xl 1280 px, 2xl 1536 px. Container scale: 3xl 768 px, 4xl 896 px, 5xl 1024 px, 6xl 1152 px, 7xl 1280 px (fetched) — [Tailwind CSS docs, Responsive design](https://tailwindcss.com/docs/responsive-design)
- Copies of Tailwind Plus marketing blocks use `max-w-7xl` (1280 px) with `px-6 lg:px-8` (24 px, then 32 px side padding) and `py-24 sm:py-32` (96 px, then 128 px section padding); one copied hero uses `py-24 sm:py-32 lg:pb-40` and a headline of `text-5xl sm:text-7xl` (48 px, then 72 px); section headings appear as `text-4xl sm:text-5xl font-semibold tracking-tight text-balance` (36 px, then 48 px). These come from unofficial copies seen in a search summary, not from the licensed source — [Tailwind Plus heroes listing](https://tailwindcss.com/plus/ui-blocks/marketing/sections/heroes); [skills.sh tailwind-marketing](https://skills.sh/peixotorms/odinlayer-skills/tailwind-marketing)
- Tailwind Plus lists 12 hero section components (search summary) — [Tailwind Plus marketing blocks](https://tailwindcss.com/plus/ui-blocks/marketing)
- A design-pattern reference gives section padding `clamp(80px, 10vw, 128px)` and a container max width of 1152 px (search summary) — [skill-wiki, Landing Page Sections](https://skill-wiki.github.io/atom/frontend-design/pattern-landing-page-sections)
- Relume's class system separates horizontal page padding (`padding-global`) from vertical section padding (`padding-section-small / medium / large`) and offers `container-small / medium / large`. A Relume community answer says `padding-section-large` resolves to 6rem (96 px) on desktop and 4rem (64 px) on mobile (search summary) — [Relume community](https://community.relume.io/x/general/msg_P2olzJTH3K0d/confusion-over-padding-section-x-classes-in-relume)

Hero variants
- Six hero structures catalogued from Tailwind's marketing blocks: simple centred (heading, subtext, button pair); split with image (two columns, text left, image right); with app screenshot (centred text, full-width screenshot below); background image (full bleed, dark overlay, white text); with phone mockup (text beside a device frame); with image tiles (text beside an asymmetric image grid) (fetched) — [skills.sh tailwind-marketing](https://skills.sh/peixotorms/odinlayer-skills/tailwind-marketing)

Type sizes and measure
- Optimal line length 50 to 75 characters, roughly 500 to 700 px at 16 px body size; body line height about 1.4 to 1.65, headings 1.0 to 1.3 (search summary) — [Refero typography reference](https://raw.githubusercontent.com/referodesign/refero_skill/f78b4eccf112d7a179b92afeafdd7e8684560ac2/references/typography.md)
- Chrome's guidance caps paragraphs at about 80 characters (`max-width: 80ch`) and recommends unitless line height such as 1.5 (search summary) — [Chrome for Developers, typography](https://developer.chrome.com/docs/modern-web-guidance/guides/visual-design/typography)
- Refactoring UI: for large headline text a line height of 1 can be fine; line height should shrink as type grows (search summary of the official preview) — [Refactoring UI preview](https://refactoringui.com/previews/line-height-is-proportional)
- One example hero rule: `h1 { font-size: clamp(2.4rem, 5vw, 4.6rem); max-width: 12ch }`, that is 38 to 74 px with a very short measure (search summary) — [skill-wiki, Landing Page Sections](https://skill-wiki.github.io/atom/frontend-design/pattern-landing-page-sections)
- Minimum body size 16 px (search summary) — [Semrush Intergalactic typography](https://developer.semrush.com/intergalactic/style/typography/typography-a11y)

Pricing table conventions
- Average number of packages on pricing pages is about 3.5, and only about 50% highlight one package as the best option (CXL-style study as summarised by HubSpot; search summary) — [HubSpot, pricing page best practices](https://blog.hubspot.com/marketing/best-practices-pricing-page)
- In the same study, participants chose the Pro plan more often when it was highlighted, most strongly when plans were ordered most expensive first (search summary) — [HubSpot](https://blog.hubspot.com/marketing/best-practices-pricing-page)
- FastSpring looked at 50 top-rated G2 companies: three plans is the most common count, the mean is four (search summary) — [FastSpring](https://fastspring.com/blog/three-tier-pricing-strategy-for-saas-is-it-still-ideal/)
- Vendor guides recommend showing monthly and annual prices with the saving stated; one recommends defaulting the toggle to annual. No study is cited for either (search summary) — [Dodo Payments](https://dodopayments.com/blogs/pricing-page-conversion-optimization)

Research-based rules (Nielsen Norman Group, fetched)
- Logo top left, not centred; a short, scannable tagline that says what the company does; replace generic welcome messages with a clear statement of the offer; show what sets you apart; use users' words, not jargon or unexplained feature names; informative images of the actual business, not stock photos or decoration — [NN/g, Homepage Design: 5 Fundamental Principles](https://www.nngroup.com/articles/homepage-design-principles/)
- Most important content above the fold, with a layout that invites scrolling; avoid "false floors" such as a full-bleed image with no sign of content below; show actual examples of content rather than only category links — [NN/g](https://www.nngroup.com/articles/homepage-design-principles/)
- Button and link labels must say what the user gets; avoid "Click Here", "Explore", "Learn More". Give the top user tasks the most visual weight; do not let every element compete. Keep primary navigation visible, not hidden behind a small Menu link — [NN/g](https://www.nngroup.com/articles/homepage-design-principles/)
- Use conventional layouts; no auto-forwarding carousels, parallax or scroll-jacking; logo, tagline and main headline visible immediately; no popups or splash screens except legally required ones — [NN/g](https://www.nngroup.com/articles/homepage-design-principles/)

Designed versus generated (2026 critiques)
- Six tells, with a page showing four or more judged "average": (1) mad-lib headline, "The AI-powered [X] for modern [Y]"; (2) centred hero with a gradient glow; (3) three identical feature cards; (4) default indigo / violet palette; (5) stock section order; (6) generic benefit copy such as "Save time. Work smarter. Scale faster." (fetched; dated 13 July 2026) — [Superdesign, Fix a generic AI landing page](https://superdesign.dev/blog/fix-generic-ai-landing-page)
- Fixes proposed by the same source: left-aligned hero with a real product screenshot on the right, no gradient, one accent colour taken from the product; 2 to 4 feature sections of different sizes, each tied to a concrete buyer task and led by a number, a before/after or a named workflow, with emoji icons removed; a named palette (for example warm off-white, near-black text, one burnt-orange accent) with one display face and one body sans; reorder sections around the actual argument (proof higher if it is stronger than features, price not buried if it is the advantage); subhead says who the page is for and gives one reason to believe — [Superdesign](https://superdesign.dev/blog/fix-generic-ai-landing-page)
- Other named tells: overused purple gradients, bento layouts used without originality (search summary) — [Search Engine Journal](https://searchenginejournal.com/7-common-ai-website-mistakes-that-are-easy-to-avoid/574196/)
- The predictable skeleton: top navbar, centred hero with call-to-action buttons, grid of feature cards; or gradient hero, three-column feature grid, rounded cards (search summary) — [UX Planet](https://uxplanet.org/how-to-spot-ai-generated-design-697aaabe76c8); [MindStudio](https://www.mindstudio.ai/blog/avoid-vibecoded-landing-pages)
- A test of AI website builders found the weakest produced layouts that were too similar and defaulted to a generic homepage even when prompted for a specific use; calls to action above the fold were a relative strength (search summary) — [Crazy Egg](https://www.crazyegg.com/blog/ai-website-builders-ux-test/)

### Inferences
- A workable Pencraft default from the verified numbers: desktop frame 1440 wide with a 1280 content container (80 px side margin) or 1152 for text-led pages; section padding 96 px, rising to 128 px on a generous page; hero headline 48 to 72 px at line height 1.0 to 1.1; body 16 to 18 px with paragraphs held to 500 to 700 px wide. The 1440 frame width and 390 mobile width themselves were not found in a citable source in this pass.
- Because the best-evidenced pricing finding is that highlighting a plan changes choice, and three or four tiers is the norm, "three or four tiers, one highlighted, saving stated next to the annual option" is a defensible default; anything stronger is vendor opinion.
- The critiques add content-level tells (headline pattern, section order, interchangeable benefit copy) that Pencraft's current list of visual tells does not cover. A landing-page skill should ask for the buyer, the single strongest proof and the real product image before drawing.
- NN/g's ban on vague button labels ("Learn more", "Explore") is directly usable as a drawing rule for static mockups.

### Gaps
- Measured anatomy of Tailwind Plus / Tailwind UI, shadcn blocks and Relume components (exact padding, grid gaps, card sizes, logo-cloud logo heights, FAQ row heights, footer column counts): pages are behind a login or were not reachable. The class values above come from unofficial copies. Not verified.
- Frame widths "1440 / 1280 / 1200, mobile 390" as current practice: no source read in this pass states them. Not verified.
- Analyses of Stripe, Linear and Vercel pages by third parties: not researched in this pass.
- Baymard and CXL primary pages: not opened; the CXL pricing study is known only through HubSpot's summary, with sample size and effect size unknown.
- Statistics often attributed to NN/g (57% or 80% of viewing time above the fold; 84% more attention above the fold) appear only in secondary blogs and conflict with each other. Not verified; do not use.
- Refactoring UI's rule against centring long text could not be confirmed from the book.
- Section-by-section inventory counts (how many variants of logo cloud, bento, testimonial, pricing, FAQ, CTA, footer in each library): only "12 heroes" was seen.

## 2. Social and ad graphics: pixel sizes, aspect ratios, safe zones

### Takeaway
Instagram is the format that changed most: the profile grid has shown 3:4 thumbnails since January 2025, so the default feed canvas is 1080 x 1350 (4:5) with everything important inside a centred strip about 1012 px wide, and 1080 x 1440 (3:4) exists only for in-app posting. For 9:16 work the safe-zone numbers differ by source and by platform; the most conservative verified set is TikTok's own template (240 top, 660 bottom, 120 each side on 1080 x 1920).

### Cited Findings

Instagram
- Feed portrait 1080 x 1350 (4:5); feed tall 1080 x 1440 (3:4), app only; square 1080 x 1080; landscape 1080 x 566 (1.91:1); story and reel 1080 x 1920 (9:16); reel cover 420 x 654; feed image ad 1440 x 1800 (4:5; 3:4 not accepted for ads); story / reels ad 1440 x 2560 (fetched; specs checked 9 October 2026) — [Sked Social](https://skedsocial.com/blog/best-instagram-image-and-video-size-recommendations.md)
- January 2025: profile grid changed from squares to vertical rectangles. May 2025: 3:4 photos keep their shape when posted from the app. Meta's publishing API (v25.0) still caps feed images at 4:5 — [Sked Social](https://skedsocial.com/blog/best-instagram-image-and-video-size-recommendations.md)
- Photos keep their resolution when 320 to 1080 px wide and between 1.91:1 and 3:4; outside that range they are cropped — [Sked Social](https://skedsocial.com/blog/best-instagram-image-and-video-size-recommendations.md)
- Grid view of a 4:5 post is the centred 1012 x 1350 area (3:4); grid thumbnail 1080 x 1440 displays at 161 x 201 (fetched; page dated 2 November 2026) — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)
- Carousels: one aspect ratio for all slides; up to 20 items in the app, 10 through the API; the first slide is the grid image; a panoramic carousel is a 3240 x 1350 master cut into three 1080 x 1350 slides; carousel ads 2 to 10 cards, at least 1080 x 1080 — [Sked Social](https://skedsocial.com/blog/best-instagram-image-and-video-size-recommendations.md)
- Story / reel safe zone, CONFLICT. Sked, citing Meta's Ads Guide: top 14% (about 269 px), bottom 35% (about 672 px), each side 6% (about 65 px). Sprout: safe area 1080 x 1610, top 14% "(250 px)" and bottom 20% "(340 px)" — [Sked Social](https://skedsocial.com/blog/best-instagram-image-and-video-size-recommendations.md); contradicted by [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)
- Profile photo: Sprout lists 320 x 320 in the body and 110 x 110 in its cheat sheet; Sked suggests uploading 1080 x 1080. Displayed as a circle — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/); [Sked Social](https://skedsocial.com/blog/best-instagram-image-and-video-size-recommendations.md)

Facebook (fetched, Sprout Social, 2 November 2026)
- Profile 320 x 320 (circle). Page cover 851 x 315, shown at 640 x 360 on mobile. Feed image 1080 x 1080 or 1440 x 1800 (4:5). Story 1080 x 1920, keep about 14% (about 250 px) clear top and bottom. Story ad 1440 x 2560 — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)

X
- Profile 400 x 400; header 1500 x 500 (3:1); post image 1600 x 900 (16:9); image ad 800 x 800 or 800 x 418; carousel ad 2 to 6 images in one ratio — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)
- CONFLICT on post image: 1600 x 900 (Sprout, SocialBee) versus 1200 x 675 (Sked) (search summary). Both are 16:9 — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/); [Sked Social guide](https://skedsocial.com/blog/the-ultimate-up-to-date-social-media-image-sizes-guide)

LinkedIn
- Personal profile 400 x 400; personal cover 1584 x 396; company logo 400 x 400; company cover 1512 x 256 (Sprout's figure; not checked against LinkedIn); post image 1200 x 1200 or 1200 x 627; carousel ad 1080 x 1080, 2 to 10 cards — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)
- LinkedIn crops anything taller than 4:5 (search summary) — [Sked Social guide](https://skedsocial.com/blog/the-ultimate-up-to-date-social-media-image-sizes-guide)
- Document (PDF carousel) posts: 1080 x 1350 (4:5) recommended by most guides, 1080 x 1080 as the alternative; these are conventions, LinkedIn publishes no page-pixel requirement. Limits reported as 300 pages and 100 MB; PDF, PPT(X) and DOC(X) accepted. One tool lists a 10-page limit, probably its own (search summary) — [ContentIn](https://contentin.io/blog/linkedin-carousel-size/); [Typefully help](https://support.typefully.com/en/articles/11970191-create-and-schedule-linkedin-pdf-carousels)
- Link preview image: at least 1200 x 627, 1.91:1 (search summary) — [LinkedIn Help](https://www.linkedin.com/help/linkedin/answer/46687)

TikTok
- Photo mode / carousel 1080 x 1920 (9:16), square and 16:9 also supported; profile 200 x 200 — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)
- In-feed ads: 9:16 recommended, minimum 540 x 960; ad profile photo 98 x 98 with the key element inside the central 66 x 66. TikTok's help page gives no safe-zone pixels; it offers downloadable template files (fetched) — [TikTok Ads Help](https://ads.tiktok.com/help/article/tiktok-auction-in-feed-ads)
- Safe zone measured from TikTok's official "In-Feed Standard LTR" template (drawn at 720 x 1280, scaled by 1.5): top 240 px, bottom 660 px, left 120 px, right 120 px on 1080 x 1920, plus a 180 px wide column on the lower right (1080 px tall from the bottom) for the like / comment / share / sound icons. With an anchor and four-line caption: bottom 1014 px, top 252 px (fetched; measured 11 November 2026) — [ManyPixels](https://www.manypixels.co/blog/social-media-design/tiktok-ad-specs)
- CONFLICT, other third-party figures: 220 top / 480 bottom / 60 left / 140 right (Revid); 108 top / 320 bottom / 60 left / 120 right (Ignite Social Media) (search summary) — [Revid](https://www.revid.ai/tiktok-safe-zone); [Ignite Social Media](https://www.ignitesocialmedia.com/content-creation/what-are-the-safe-zones-for-tiktoks-and-instagram-reels/)
- Carousel ad images: 720 x 1280, 640 x 640 or 1200 x 628; 2 to 35 images — [ManyPixels](https://www.manypixels.co/blog/social-media-design/tiktok-ad-specs)

YouTube
- Thumbnail: 3840 x 2160 for videos (16:9), 2160 x 3840 for Shorts (9:16), 1:1 for podcast playlists; minimum 640 px wide; file limit 2 MB on mobile and 50 MB on desktop (fetched, official; no revision date on page) — [YouTube Help](https://support.google.com/youtube/answer/72431)
- Older guides still give 1280 x 720 as the thumbnail size (search summary) — [SocialBee](https://socialbee.com/blog/social-media-image-sizes/)
- Channel banner 2560 x 1440 recommended, minimum 2048 x 1152, 6 MB; text-and-logo safe area 1235 x 338 (Sked and Sprout's body; Sprout's table says 1235 x 388, an internal inconsistency) — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/); [Sked Social guide](https://skedsocial.com/blog/the-ultimate-up-to-date-social-media-image-sizes-guide)

Pinterest
- Pin 1000 x 1500 (2:3); profile 280 x 280; cover 800 x 450; carousel ad 2 to 5 images at 1:1 or 2:3 — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)
- Pinterest does not recommend images smaller than 600 x 900; pins taller than about 1000 x 2100 can be cut off in the feed; square 1000 x 1000 is supported (search summary; third-party guides describing Pinterest's guidance) — [Tailwind app](https://www.tailwindapp.com/blog/pinterest-image-size)

Threads
- Square 1080 x 1080; vertical 1440 x 1800 (4:5); carousel 2 to 20 items; profile 320 x 320 — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)

Bluesky
- Bluesky publishes no official dimensions. Recommended post image 1080 x 1350 or 1080 x 1080, up to 4 images, 2 MB each; profile 1000 x 1000 (cheat sheet says 400 x 400); banner 3000 x 1000, 3:1 (cheat sheet says 1500 x 1500, which is not 3:1) — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)
- April 2026: Bluesky raised post-image support to 2 MB per image and up to 4000 x 4000 px (search summary) — [Influencer Marketing Hub](https://influencermarketinghub.com/social-media-image-sizes-guide/)

Snapchat
- Story / single image ad 1080 x 1920 (ad minimum 720 x 1280); keep the top 150 px and bottom 330 px clear — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)

Cross-platform
- 1080 x 1080 displays as a feed post on Instagram, Facebook, LinkedIn, Bluesky and Threads without automatic cropping (search summary) — [Sprout Social](https://sproutsocial.com/insights/social-media-image-sizes-guide/)

Open Graph / link previews
- 1200 x 630 is a practical cross-platform default, not part of the Open Graph protocol, which defines only an image URL (search summary) — [DEV, OG image sizes 2026](https://dev.to/corbanware/og-image-sizes-the-complete-guide-for-2026-4l6d)
- X `summary_large_image`, CONFLICT: 1200 x 600 (2:1), 1200 x 628 (1.91:1) and 1200 x 675 (16:9) are each given by different guides (search summary) — [Screenhance](https://screenhance.com/blog/og-image-size-guide); [Krumzi](https://www.krumzi.com/blog/og-image-size-cheat-sheet-1200x630)
- GitHub repository social preview: 1280 x 640 recommended, at least 640 x 320, under 1 MB, PNG / JPG / GIF (fetched, official) — [GitHub Docs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview)

Google display and Demand Gen (fetched, official)
- Image assets: square 1200 x 1200 (minimum 300 x 300); horizontal 1.91:1 at 1200 x 628 (minimum 600 x 314); vertical 4:5 at 960 x 1200 (minimum 480 x 600); vertical 9:16 at 1080 x 1920 (minimum 600 x 1067); image file up to 5 MB. Logo 1:1 at 1200 x 1200 (minimum 128 or 144, the page gives both), 150 KB — [Google Ads Help](https://support.google.com/google-ads/answer/17140672)
- A square logo loses about 21.46% of its area when shown as a circle in Gmail (about 5.36% per corner) — [Google Ads Help](https://support.google.com/google-ads/answer/17140672)
- Uploaded display ads, fixed sizes: 300 x 250 (medium rectangle), 336 x 280 (large rectangle), 728 x 90 (leaderboard), 970 x 90 (large leaderboard), 160 x 600 (wide skyscraper), 300 x 600 (half page), 320 x 50 (mobile leaderboard); JPG, PNG or non-animated GIF; 150 KB maximum — [Google Ads Help](https://support.google.com/google-ads/answer/17140672)
- Accompanying text limits: headline 40 characters, description 90, business name 25 — [Google Ads Help](https://support.google.com/google-ads/answer/17140672)
- Responsive display ads take 1200 x 628, 1200 x 1200 and 900 x 1600 (9:16), up to 15 images, 5120 KB each; Google states that Display campaigns are moving to Demand Gen (search summary) — [Google Ads Help, responsive display best practices](https://support.google.com/google-ads/answer/9823397)

Podcast cover (fetched, official)
- Apple Podcasts show cover 3000 x 3000 px (RSS submissions accept 1400 x 1400 up to 3000 x 3000, largest preferred); PNG or JPG; no transparency or alpha channel; keep key elements inside the template's art safe area because the play button, labels and subscription offers can cover the edges; keep the title large and legible at small sizes; limit extra text and logos; no Apple logos or hardware imagery — [Apple Podcasts for Creators](https://podcasters.apple.com/support/5514-show-cover-template)

### Inferences
- For a static tool the safest universal rule for 9:16 is to draw with TikTok's margins (240 top, 660 bottom, 120 sides), because they are the largest verified set and contain Instagram's Meta-derived zone on the top and sides (269 px top is slightly larger than 240, so use 270 top to satisfy both). That leaves a working area of roughly 840 x 990.
- Sprout's Instagram story numbers are arithmetically inconsistent (14% of 1920 is 269, not 250; 1920 minus 250 minus 340 is 1330, not 1610), so Sked's Meta-derived figures are the more trustworthy of the two.
- For a 4:5 Instagram post, the centred 1012 x 1350 strip means a 34 px loss on each side in the grid; a 64 px side margin for text comfortably covers it.
- YouTube's official thumbnail figure is now 3840 x 2160; 1280 x 720 remains valid as the same 16:9 ratio at lower resolution, and the 2 MB mobile limit favours it for JPEG export.
- A 1200 x 630 image with content held in the central region is the only choice that works across Facebook, LinkedIn and X given the unresolved X ratio conflict.
- Google's 21.46% circle-crop figure is a usable general rule for any logo or avatar that will be shown in a circle: keep the mark inside the inscribed circle.

### Gaps
- Official Meta help pages for Instagram and Facebook were not opened; all Meta figures are third-party (Sked cites Meta's Ads Guide and API v25.0).
- X official specifications, including header safe area and where the profile photo overlaps the header: not found.
- Facebook cover mobile safe zone in pixels: not found.
- Pinterest's own help page: not opened.
- IAB's own standard ad unit portfolio page: not opened; the fixed sizes are from Google's list.
- Text-size and contrast rules for small-screen legibility of social graphics (minimum px for headline and body on a 1080 canvas): no reliable source found in this pass.
- Carousel craft conventions beyond platform limits (cover slide, numbering, swipe cue, closing call to action): not researched.
- Twitch panel and offline banner sizes: not found.

## 3. Slides: canvas, margins, type minimums, archetypes, one-idea rules, pitch-deck order

### Takeaway
The recognised sources agree on behaviour rather than numbers: one idea per slide, the conclusion stated as the title, text large enough for the back of the room, and very few slides. The only firm numeric rules found are Kawasaki's 30-point minimum and the accessibility floor of 24 points; margins and safe areas for a 1920 x 1080 slide are not specified by any source read.

### Cited Findings
- Kawasaki's 10/20/30 rule: ten slides, no more than twenty minutes, no font smaller than thirty points; written for investor pitches (search summary; original page returned HTTP 403) — [Guy Kawasaki](https://guykawasaki.com/the_102030_rule/); [Inc.](https://www.inc.com/jessica-stillman/presentations-guy-kawasaki-10-20-30-rule.html)
- Kevin Hale (Y Combinator): slides must be legible, simple and obvious. Legible: large, bold text in a simple font, strong contrast, text placed near the top of the slide (easier to read from a distance), no small, thin or light-grey text, and never let a screenshot dictate text size or placement. Simple: one idea per slide; pick 5 to 7 key ideas for the whole deck. Obvious: show the slide to a stranger and revise if they do not state the idea at once; for charts, state the conclusion in the headline or caption; remove excess text, caveats, branding, untitled photos, animations, memes; avoid complicated diagrams. Screenshots are "almost always illegible, complex, and non-obvious" (fetched; article dated 2015) — [Y Combinator, How to Design a Better Pitch Deck](https://www.ycombinator.com/blog/how-to-design-a-better-pitch-deck)
- YC seed deck template, slide order: title (company name and one-line description) → problem → solution → traction → second traction or metric slide → unique insight ("what makes the company special") → business model → market and future growth → team → the ask (amount raised and what it funds; no valuation or terms). The template's own slide titles are full-sentence claims, for example "Founders build bad seed decks" (fetched) — [YC seed deck template](https://docs.google.com/presentation/d/17nFIwCyf2Kz-Ao5HGnmvNZ74L8eSKA2C2Qdaoe-47OM/htmlpresent)
- Sequoia's outline, in order: company purpose (one sentence) → problem → solution → why now → market size → competition → product → business model → team → financials (search summary of the official article) — [Sequoia Capital, Writing a Business Plan](https://sequoiacap.com/article/writing-a-business-plan)
- YC's template puts traction near the front, unlike Sequoia's order; one critique advises deleting Sequoia's "company purpose" slide and moving team up for seed rounds (search summary) — [Scott Sage on Medium](https://medium.com/@scott_sage/fundraising-why-you-shouldnt-just-copy-sequoia-s-pitch-deck-template-4b32ac60d93a)
- Consulting-style slides: the action title is a complete sentence stating the takeaway, not a topic; a reader should follow the argument from the titles alone; one message per slide, and a crowded slide is split. Minto's Pyramid Principle puts the conclusion first (search summary) — [think-cell](https://www.think-cell.com/en/resources/content-hub/using-the-pyramid-principle-to-build-better-powerpoint-presentations); [Deckary](https://deckary.com/blog/consulting-quality-slides)
- Text-density rules cited by guides conflict: 5 x 5 (five words per line, five lines) versus 6 x 6 (search summary) — [Deckary](https://deckary.com/blog/consulting-slide-standards)
- Tufte's critique, as reported by Farkas: bullet points leave the presenter's reasoning unspecified and let key points hide; projected slides carry less data than paper handouts (search summary; secondary) — [Farkas, Toward a Better Understanding of PowerPoint](https://faculty.washington.edu/farkas/dfpubs/Farkas-Toward%20A%20Better%20Understanding%20of%20PowerPoint.pdf)
- Type minimums: 24 pt is the floor in university accessibility guidance; others recommend 28 pt or more for body text and 32 to 40 pt in many talks; a design firm allows 12 to 20 pt in sales or training decks meant to be read; captions or footnotes 14 to 16 pt minimum. Apparent size depends on the typeface (search summary) — [UGA accessible slides](https://dash.uga.edu/create-accessible-content/slides/); [BrightCarbon](https://www.brightcarbon.com/blog/presentation-font-size); [Beautiful.ai](https://www.beautiful.ai/blog/what-font-size-is-best-for-presentations)

### Inferences
- Point sizes in these sources assume the classic 13.33 x 7.5 inch slide, on which 1 pt equals 2 px at 1920 x 1080. On a 1920 x 1080 Pencraft frame that makes Kawasaki's 30 pt about 60 px, the 24 pt floor about 48 px, and a 14 pt footnote about 28 px. This conversion is my arithmetic, not a sourced rule.
- Two deck types need different rules: a presented deck (48 px minimum, 60 px preferred, a handful of words) and a reading deck sent by email (24 to 40 px body, action titles, denser evidence). Hale's "no screenshots" is a rule for a room, as one guide notes.
- "Title near the top, stated as a full-sentence claim" is supported by both Hale and consulting practice, and is a concrete drawing rule: the title block is the conclusion, the body is the evidence.
- The slide archetype list requested (title, section, statement, one chart, comparison, quote, agenda, closing) can be assembled from these sources but no single source read defines it.

### Gaps
- Duarte's glance test (commonly quoted as three seconds): the Duarte page returned 404 and no search result confirmed it. Not verified.
- Reynolds, "Presentation Zen": not researched.
- Margins and safe areas for 1920 x 1080 in pixels: no source gives them. The one relevant result describes broadcast "safe title" areas without numbers.
- 4:3 canvas sizes: not sourced (1024 x 768 and 1440 x 1080 are common but uncited here).
- Kawasaki's list of ten slide topics and his "divide the oldest audience member's age by two" heuristic: original page blocked. Not verified.
- A "two-line maximum" for action titles: not confirmed by any source.

## 4. Print: sizes, bleed, safe margin, type and line minimums, RGB caveats, poster hierarchy

### Takeaway
Sizes are stable and well sourced (ISO 216, North American sizes, regional business cards); the working rules are 3 mm (0.125 in) bleed on every side, important content 3 to 5 mm inside the trim, 300 dpi at final size, 6 pt minimum type (8 pt when reversed out of a dark background) and 0.25 pt minimum line weight. Printers differ on bleed and safe figures, so their own template always wins.

### Cited Findings

Paper sizes (fetched)
- ISO A series in mm: A0 841 x 1189; A1 594 x 841; A2 420 x 594; A3 297 x 420; A4 210 x 297; A5 148 x 210; A6 105 x 148; A7 74 x 105. Also B4 250 x 353; B5 176 x 250; C4 229 x 324; C5 162 x 229; C6 114 x 162; DL envelope 110 x 220 — [Wikipedia, Paper size](https://en.wikipedia.org/wiki/Paper_size)
- All ISO sizes share the ratio 1 : 1.4142 (square root of 2); each size is half the area of the next; A0 is 1 square metre. Tolerances: plus or minus 1.5 mm up to 150 mm, 2 mm from 150 to 600 mm, 3 mm above 600 mm — [Wikipedia, Paper size](https://en.wikipedia.org/wiki/Paper_size)
- North American: Letter 8.5 x 11 in (216 x 279 mm); Legal 8.5 x 14 in (216 x 356 mm); Tabloid 11 x 17 in (279 x 432 mm); Half Letter 5.5 x 8.5 in (140 x 216 mm) — [Wikipedia, Paper size](https://en.wikipedia.org/wiki/Paper_size)

Pixels at 300 dpi, computed from the sizes above (trim size, no bleed)
- A0 9933 x 14043; A1 7016 x 9933; A2 4961 x 7016; A3 3508 x 4961; A4 2480 x 3508; A5 1748 x 2480; A6 1240 x 1748; A7 874 x 1240; Letter 2550 x 3300; Legal 2550 x 4200; Tabloid 3300 x 5100; Half Letter 1650 x 2550; 18 x 24 in poster 5400 x 7200; 24 x 36 in poster 7200 x 10800. Bleed of 3 mm is 35 px per side; 0.125 in is 37.5 px per side — computed from [Wikipedia, Paper size](https://en.wikipedia.org/wiki/Paper_size)

Business cards (fetched, Vistaprint)
- US / Canada 3.5 x 2 in (88.9 x 50.8 mm), 1050 x 600 px at 300 dpi; UK / Europe 85 x 55 mm, 1004 x 650 px; Japan 91 x 55 mm; Australia 90 x 55 mm; China 90 x 54 mm; Vistaprint square 63.7 x 63.7 mm, 752 x 752 px; mini 76.2 x 30 mm, 900 x 354 px; rounded corners use a 0.25 in radius — [Vistaprint, business card dimensions](https://www.vistaprint.com/hub/business-card-dimensions)
- Vistaprint standard card: full bleed 3.61 x 2.11 in (1083 x 633 px), trim 3.5 x 2 in (1050 x 600), safe area 3.36 x 1.86 in (1008 x 558). That is a bleed of about 0.055 in (1.4 mm) and a safe margin of 0.07 in (1.8 mm), smaller than the general 3 mm rule; the same page's Photoshop instructions give safe guides 0.125 in inside the edge, an internal inconsistency — [Vistaprint](https://www.vistaprint.com/hub/business-card-dimensions)
- Minimums on a card: contact details at least 8 pt, names and headings 10 to 12 pt or larger; QR code at least 0.8 x 0.8 in (20 x 20 mm) inside the safe zone with a clear quiet zone — [Vistaprint](https://www.vistaprint.com/hub/business-card-dimensions)
- Postcards (Vistaprint): 4.2 x 5.5 in, 4 x 6 in, 5 x 7 in, 6 x 11 in. Greeting / holiday cards: 5 x 7, 6 x 9 and 4 x 8 in — [Vistaprint](https://www.vistaprint.com/hub/business-card-dimensions)

Bleed, safe margin, type and line minimums (search summaries of printers' specification pages)
- Bleed: 3 mm (0.125 in) on all four sides is the common standard; one packaging spec asks for a 3 mm minimum and ideally 5 mm — [Jukebox Print](https://www.jukeboxprint.com/business-cards/business-card-rules); [BoxMart artwork specification](https://cdn.shopify.com/s/files/1/0263/0788/7138/files/BoxMart_Artwork_Specification_2023.pdf)
- Safe margin: keep important content 3 mm inside every trim edge; "usually 3 to 5 mm" — [instantprint](https://instantprint.co.uk/artwork-and-design); [Jukebox Print](https://www.jukeboxprint.com/business-cards/business-card-rules)
- Minimum type: 6 pt for dark text on light, 8 pt for reversed (light on dark) text, larger or bolder for thin or intricate faces; one printer allows 4 pt on flat or business-card print and 6 pt for large format — [BoxMart artwork specification](https://cdn.shopify.com/s/files/1/0263/0788/7138/files/BoxMart_Artwork_Specification_2023.pdf); [Jukebox Print help](https://support.jukeboxprint.com/en/articles/4214205-what-is-the-smallest-font-size-i-should-use)
- Minimum line weight: 0.25 pt for offset, 0.4 pt for digital; 0.5 pt recommended, 0.75 pt if in doubt on uncoated stock; reversed lines 0.5 pt — [Print Arabia](https://printarabia.ae/help-print/creating-data/text-size-line-weights); [Greener Printer help](https://greenerprinterhelp.freshdesk.com/support/solutions/articles/63000281721-what-is-the-finest-detail-i-should-include-in-my-printed-design-)
- Resolution: 300 dpi at final size; convert fonts to outlines before export — [Jukebox Print](https://www.jukeboxprint.com/business-cards/business-card-rules)

Posters at viewing distance (search summaries)
- A research poster should be readable from 6 feet, which typically means letters at least 1 inch tall — [APTA CSM 2025 poster format guidelines](https://csm.apta.org/contentassets/de3ddc3607b646359c87f940ce5c649e/apta-csm-2025-poster-format-guidelines.pdf)
- Canadian government signage table: about 5 mm of character height per metre of viewing distance — [Treasury Board of Canada, viewing distance and character size](https://www.tbs-sct.canada.ca/fip-pcim/images/man/4-3_tbl02-450w-eng.html)
- Doubling the reading distance requires doubling character height — [Science Buddies](https://www.sciencebuddies.org/science-fair-projects/science-fair/display-board-fonts)
- One university handout suggests title 85 pt and body 24 pt for a specific poster size; another sets 28 pt as the minimum — [UC Davis poster design principles](https://urc.ucdavis.edu/sites/g/files/dgvnsk3561/files/inline-files/General%20Poster%20Design%20Principles%20-%20Handout.pdf); [University of York poster guide](https://subjectguides.york.ac.uk/posters/text)

Garment printing
- Printful's standard t-shirt front print area is 12 x 16 in; maximum placement ranges from 10 x 12 in to 12 x 16 in by product; selected products now allow 15 x 18 in; recommended file resolution 150 to 300 dpi (search summary) — [Printful, t-shirt design placement guide](https://printful.com/blog/t-shirt-design-placement-guide); [Printful help](https://help.printful.com/hc/en-us/articles/18532732956828)

### Inferences
- At 150 dpi a 12 x 16 in shirt print is 1800 x 2400 px; at 300 dpi, 3600 x 4800 px (computed).
- Pencraft's RGB-only output matters most for offset print: saturated RGB blues, greens and oranges will shift when the printer converts to CMYK, and a "rich black" cannot be specified. A print skill should tell the model to say so, to avoid neon and pure RGB primaries for brand colours, and to recommend vector PDF with bleed drawn into the frame. This is general print knowledge, not drawn from a source read in this pass.
- Because Pencraft exports vector PDF, a print frame can be drawn at 1 px per point (A4 as 595 x 842, Letter as 612 x 792) and still print sharply for text and shapes, with only placed images needing 300 dpi. The 300 dpi pixel table is needed for PNG / JPEG export. This is my reasoning and should be checked against how Pencraft maps px to PDF units.
- Taking 5 mm per metre: a poster headline to be read from 5 m needs about 25 mm capital height (roughly 100 pt type), and body copy read from 1.5 m needs about 7.5 mm (roughly 30 pt). These conversions are mine.
- The safe default for an unknown printer is 3 mm bleed and 5 mm safe margin; Vistaprint's tighter template shows printers vary, so the skill should name the printer template as the authority.

### Gaps
- Moo and Flyeralarm specification pages: not read (Moo URL returned 404).
- DL flyer (99 x 210 mm), folds and brochure panel widths, book cover trim sizes and spine calculation, packaging label and sticker sizes, menus, invitations, certificates and resume conventions: not researched or not found. The Wikipedia page gives only the DL envelope, 110 x 220 mm.
- US poster sizes 18 x 24 and 24 x 36 in are common but not confirmed by a source read here.
- Müller-Brockmann grid systems for posters: not researched.
- RGB versus CMYK conversion behaviour at specific printers (which accept RGB PDF, which convert automatically): not researched.
- A sourced viewing-distance formula in "inch per 10 feet" form was not found; the two cited rules differ (1 inch per 6 feet versus about 1 inch per 16 feet), because one describes comfortable reading of body text at a poster session and the other minimum legibility of signage.

## 5. Email: width, mobile, fonts, buttons, dark mode, anatomy

### Takeaway
The 600 px width convention is confirmed by Mailchimp and several others, with a 100 to 102 KB HTML weight ceiling before Gmail clips the message; beyond that, the major vendors' own pages give few hard numbers, and button and type sizes come from secondary guides that disagree.

### Cited Findings
- Mailchimp templates are no wider than 600 px so that they work in most email clients (search summary) — [Mailchimp, About Email Template Widths (via QuickBooks help)](https://quickbooks.intuit.com/learn-support/en-us/help-article/custom-templates/email-template-widths/L6JJG4wPh_US_en_US); [Mailchimp template reference](https://templates.mailchimp.com/getting-started/html-email-basics/)
- Other guides: 600 px recommended, 650 px maximum; some allow 600 to 800 px (search summary) — [Hostinger](https://www.hostinger.com/tutorials/email-design-best-practices/); [Cloudera](https://docs.cloudera.com/data-visualization/7/ref-email-dev-rec/topics/viz-email-design-best-practices.html)
- Anatomy named by Litmus: header (often the logo), body, call to action (size, shape, colour and position of buttons), images, footer (social links, unsubscribe, legal). Avoid image-only emails; write short descriptive alt text; the unsubscribe link must be easy to see (fetched; published 21 November 2024) — [Litmus, email design best practices](https://www.litmus.com/blog/email-design-best-practices)
- Subscribers spend an average of 8.97 seconds on an email, so headings, bold, colour and bullets are needed as anchors — [Litmus](https://www.litmus.com/blog/email-design-best-practices)
- Dark mode: more than a third of subscribers read in dark mode; clients behave in three ways: no change, partial colour inversion, full colour inversion — [Litmus](https://www.litmus.com/blog/email-design-best-practices)
- Fonts: web-safe fonts are reliable but limit design; web fonts may not display. Example stack `'Roboto', 'Arial', sans-serif` at 16 px (fetched) — [Litmus](https://www.litmus.com/blog/email-design-best-practices); [Email on Acid](https://www.emailonacid.com/blog/article/email-development/email-development-best-practices-2/)
- Mailchimp's own readability example: "12px Comic Sans = bad. 16px Georgia = good" (search summary) — [Mailchimp template design guide](https://templates.mailchimp.com/design/layout-and-purpose/)
- Mobile: 480 px is no longer a safe assumption for all phones; design mobile-first in a single column (fetched) — [Email on Acid](https://www.emailonacid.com/blog/article/email-development/email-development-best-practices-2/)
- Weight: emails above 100 KB score worse in spam tests and may be clipped by Gmail; other sources give 102 KB as the desktop Gmail limit; images are loaded externally and do not count toward it — [Email on Acid](https://www.emailonacid.com/blog/article/email-development/email-development-best-practices-2/); [Klaviyo help](https://help.klaviyo.com/hc/en-us/articles/115000591251-Why-is-my-email-being-clipped-)
- Outlook on iOS clips emails taller than 5000 px (search summary) — [Really Good Emails support](https://support.reallygoodemails.com/hc/en-us/articles/24963912931602-Email-Size-Gmail-Clipping)
- Buttons, CONFLICT: tap targets at least 46 x 46 px (Omnisend); button 200 to 300 px wide by 50 to 75 px high (Zoho, Tabular) (search summary) — [Omnisend help](https://support.omnisend.com/en/articles/1061843-basics-of-email-html); [Zoho ZeptoMail](https://www.zoho.com/zeptomail/articles/email-template-size.html)
- Preview / preheader text 50 to 100 characters (search summary) — [Tabular](https://tabular.email/blog/email-template-size-width-and-height)
- Tools email designers use: Photoshop 37%, Figma 23%, Illustrator 22%, Canva 18%, InDesign 14% — [Litmus](https://www.litmus.com/blog/email-design-best-practices)

### Inferences
- A Pencraft email mockup is a picture of an email, not the email; the useful rules are therefore layout ones: a 600 px wide single column inside a wider background frame, a 375 to 390 px mobile variant in one column, live-text-looking headings in a system-like face, one primary button at least 46 px tall, and a footer with unsubscribe and address.
- Because partial inversion flips light backgrounds but not images, logos drawn on a transparent background with dark ink disappear in dark mode. Drawing a dark-mode variant frame beside the light one is the practical answer. This follows from Litmus's three behaviours but the specific logo advice was not found in a source read here.
- Total height matters: with clipping near 5000 px in one client and 9 seconds of attention, a design much taller than roughly three screens of a phone is working against the evidence.

### Gaps
- Litmus, Mailchimp and Email on Acid pages read give no figures for heading or body sizes, line height, retina image scale, button height or preheader length; those remain vendor-blog numbers.
- Really Good Emails design guidance (as opposed to its support note): not found.
- Share of opens by client and by dark mode for 2026: only "more than a third" from a 2024 Litmus page.
- The widely quoted 44 x 44 px tap target for email buttons: no source found in this pass.
- List of web-safe fonts: not sourced.

## 6. Store and listing assets

### Takeaway
All four stores publish exact figures and these were read from the official pages: App Store screenshots are now driven by 1320 x 2868 (large) and 1206 x 2622 (medium) iPhone classes and 2064 x 2752 iPad; Google Play needs a 512 x 512 icon and a 1024 x 500 feature graphic; Steam enlarged its capsules in August 2024 and rejects the old sizes; the Chrome Web Store wants 1280 x 800 screenshots and a 440 x 280 tile.

### Cited Findings

Apple App Store (fetched, official)
- General: 1 to 10 screenshots per device size; JPEG or PNG; no alpha channel or transparency; if the interface is the same across sizes only the highest required resolution is needed and App Store Connect scales it down — [App Store Connect Help, screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications)
- iPhone, Dynamic Island large: 1320 x 2868, 1290 x 2796 or 1260 x 2736 (and landscape). Dynamic Island medium: 1206 x 2622 or 1179 x 2556; the page as read marks at least one of this class as required for iPhone. Face ID large: 1284 x 2778 or 1242 x 2688, required if the large Dynamic Island set is not supplied. Face ID medium: 1170 x 2532, 1125 x 2436, 1080 x 2340. Home button large 1242 x 2208; medium 750 x 1334 — [App Store Connect Help](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications)
- The page as read also lists a foldable "iPhone Duo" class (outer 1398 x 2034, inner 2007 x 2853), required from April 2027 for apps built with the iOS 27.1 SDK or later — [App Store Connect Help](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications)
- iPad 13 inch: 2064 x 2752 or 2048 x 2732, required if the app runs on iPad. iPad 11 inch: 1668 x 2420, 1668 x 2388, 1640 x 2360, 1488 x 2266, 1512 x 2316 — [App Store Connect Help](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications)
- Mac: 16:10 at 1280 x 800, 1440 x 900, 2560 x 1600 or 2880 x 1800. Apple TV 1920 x 1080 or 3840 x 2160. Vision Pro 3840 x 2160. Apple Watch: 422 x 514, 410 x 502, 416 x 496, 396 x 484, 368 x 448, 312 x 390 — [App Store Connect Help](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications)
- CONFLICT: third-party guides and an earlier reading of Apple's page describe the 6.9 inch set (1320 x 2868 or 1290 x 2796) as the required iPhone set unless a 6.5 inch set is supplied, which differs from the "medium is required" reading above (search summary) — [Adalo](https://studio.adalo.com/blog/app-store-screenshot-sizes-2026)

Google Play (fetched, official)
- App icon 512 x 512, 32-bit PNG with alpha, up to 1024 KB. Feature graphic 1024 x 500, JPEG or 24-bit PNG without alpha; keep the focal point and key elements (logo, name, slogan) towards the centre and out of the cut-off zones, which the page shows only as an image — [Play Console Help](https://support.google.com/googleplay/android-developer/answer/9866151)
- Screenshots: at least two overall; each side 320 to 3840 px; longer side no more than 2.3 times the shorter; phones 2 to 8 at 9:16 or 16:9; for store recommendations at least four at 1080 px minimum (1080 x 1920 or 1920 x 1080). Tablets and Chromebooks: at least 4, sides 1080 to 7680 px. Wear OS 1:1, minimum 384 x 384. Android TV banner 1280 x 720. Android XR 8:5, 3840 x 2400 recommended — [Play Console Help](https://support.google.com/googleplay/android-developer/answer/9866151)
- Content guidance: use vibrant colours and avoid pure white, black or dark grey, which blend into the store background; no device imagery; no store badges; avoid ranking or promotional words such as "Best", "#1", "Free" — [Play Console Help](https://support.google.com/googleplay/android-developer/answer/9866151)

Steam (fetched, official)
- Store: header capsule 920 x 430; small capsule 462 x 174; main capsule 1232 x 706; vertical capsule 748 x 896; screenshots at least 1920 x 1080, 16:9; page background 1438 x 810 (subtle, not too bright). Library: capsule 600 x 900; hero 3840 x 1240 (artwork only, no logo); logo PNG 1280 wide and / or 720 tall; library header 920 x 430. Icons: shortcut 256 x 256; app icon 184 x 184 JPG. Events: cover 800 x 450; header 1920 x 622 — [Steamworks documentation, store assets](https://partner.steamgames.com/doc/store/assets)
- Capsules carry only the game's logo and artwork. Since August 2024 the larger sizes apply and the old dimensions are no longer accepted (the header was previously 460 x 215) — [Steamworks documentation](https://partner.steamgames.com/doc/store/assets); [Steam Page Analyzer](https://www.steampageanalyzer.com/blog/steam-capsule-sizes)

Chrome Web Store (fetched, official)
- Icon 128 x 128 PNG with the artwork at 96 x 96 and 16 px of transparent padding per side; must work on light and dark backgrounds; no border; front-facing; non-transparent images are framed with 12 px rounded corners; a circular icon is about 112 px in diameter — [Chrome for Developers, Web Store images](https://developer.chrome.com/docs/webstore/images)
- Screenshots: 1 to 5, at 1280 x 800 (preferred) or 640 x 400, square corners, no padding; all are currently downscaled to 640 x 400. Small promotional tile 440 x 280 (items without it are listed after those with it). Marquee tile 1400 x 560 — [Chrome for Developers](https://developer.chrome.com/docs/webstore/images)
- Promotional image guidance: avoid text, the image must still work at half size, assume a light grey background, use saturated colours, fill the whole region, show the brand rather than a screenshot — [Chrome for Developers](https://developer.chrome.com/docs/webstore/images)

Other listings
- GitHub social preview 1280 x 640 (minimum 640 x 320, under 1 MB) (fetched, official) — [GitHub Docs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview)
- Product Hunt: gallery images most often given as 1270 x 760; thumbnail 240 x 240 square; the first gallery image is used as the social preview. Conflicts: file limit 3 MB versus 5 MB; one source lists a 240 x 180 thumbnail (search summary; third-party) — [Screenhance](https://screenhance.com/blog/product-hunt-gallery-size)
- Favicon set: `favicon.ico` 32 x 32 (optionally with a 16 x 16 layer); `icon.svg`; `apple-touch-icon.png` 180 x 180; manifest icons 192 x 192 and 512 x 512; maskable icon on a 512 x 512 canvas with the logo inside a central circle of 409 x 409 (fetched; page dated 21 January 2026) — [Evil Martians](https://evilmartians.com/chronicles/how-to-favicon-in-2021-six-files-that-fit-most-needs)
- YouTube channel banner and thumbnail: see section 2 — [YouTube Help](https://support.google.com/youtube/answer/72431)

### Inferences
- A store-assets skill can be a lookup table plus four rules that recur across stores: no transparency in screenshots, key content centred (crops and overlays differ by surface), little or no text on small promotional tiles, and colours that do not merge with a white or dark store background.
- Since App Store Connect scales down, drawing iPhone screenshots at 1320 x 2868 and iPad at 2064 x 2752 covers the Apple requirement in most cases; the conflict over which class is formally "required" should be checked in App Store Connect before relying on one set.
- Steam's four store capsules have four different ratios (2.14:1, 2.66:1, 1.74:1, 0.83:1), so each needs its own composition of the same logo and key art rather than a crop of one master.

### Gaps
- Apple app icon size (1024 x 1024 is the long-standing figure) and Apple's marketing rules for screenshot captions: not read in this pass.
- Google Play feature graphic cut-off zones in pixels: shown only as an image on the official page.
- Steam's separate "Graphical Asset Rules" page (what text is prohibited on capsules) and bundle image sizes: not read.
- Product Hunt official specifications: none found; all figures are third-party and conflict.
- Twitch and YouTube overlay sizes (panels, offline banner, stream overlay safe areas): not found.
- The "iPhone Duo" entry is reported exactly as the fetch of Apple's page returned it; it was not cross-checked against a second source.

## 7. Demand evidence: which formats are requested most

### Takeaway
Hard evidence is thin. The single usable survey figure puts social media content first by a wide margin (69% of creators), with presentations at 49%; template-market commentary agrees that social media graphics and presentations lead, followed by print pieces such as resumes and event printables. No template-library category counts could be obtained.

### Cited Findings
- Canva's 2026 design predictions ("Imperfect by Design"), survey of 1,000 creators: the most common tasks for which creators use AI are social media content 69%, video 51%, presentations 49%, art or illustrations 40%, data visualisation 37%. These describe AI-assisted tasks, not all Canva usage (fetched; article dated 11 December 2025) — [TechRound](https://techround.co.uk/news/canva-2026-design-predictions-what-think/)
- Canva's template pages are its main source of search traffic; resume templates are cited as a strong example, with 219 child template pages most of which are said to draw over 1.3 million in search traffic (search summary; third-party SEO analysis, figures unverified) — [LinkedIn Pulse, Canva's SEO strategy breakdown](https://www.linkedin.com/pulse/canvas-seo-strategy-breakdown-w-content-saleh-ahmed-rid8e)
- Commentary on the market for selling Canva templates names business printables, social media templates and event printables as the most popular, and describes the market as led by professional presentations for corporate and startup use and high-volume social media bundles (search summary; sourcing unclear) — [Vizologi](https://vizologi.com/what-sells-most-from-canva/); [Accio](https://www.accio.com/business/best_selling_templates_on_canva)
- Canva's user base by sector: education about 15%, non-profits 7%, marketing 6% (search summary; third-party aggregator) — [Style Factory](https://www.stylefactoryproductions.com/blog/canva-statistics)
- Email designers' tools: Figma 23%, Canva 18% (Litmus survey), showing that email layouts are already designed in general canvas tools — [Litmus](https://www.litmus.com/blog/email-design-best-practices)
- The number of fixed specifications each family carries, from this research: social and ad graphics have by far the most distinct sizes and the most changes in 2025 to 2026 (Instagram grid January 2025, 3:4 posts May 2025, Bluesky limits April 2026, Steam capsules August 2024) — [Sked Social](https://skedsocial.com/blog/best-instagram-image-and-video-size-recommendations.md); [Influencer Marketing Hub](https://influencermarketinghub.com/social-media-image-sizes-guide/); [Steamworks](https://partner.steamgames.com/doc/store/assets)

### Inferences
- Ranking by the evidence available: social graphics first, slides second, then print (resumes, flyers, invitations, business cards), then landing pages, with email and store assets as narrower but specification-heavy niches. Landing pages rank lower in consumer template demand but are probably over-represented among requests to an AI design tool, given the volume of 2026 writing about generated landing pages; that last point is an impression from the search results, not a measurement.
- Value of a skill to Pencraft is not the same as demand: the families where a model without the skill produces unusable output are those with hard numbers it cannot guess (social safe zones, store asset sizes, print bleed). Slides and landing pages fail on craft rather than on dimensions.
- Resumes stand out as a single print format with disproportionate search demand and deserve a place inside a print skill.

### Gaps
- Canva's own ranking of most-created or most-searched design types: none published that could be found. Not verified.
- Adobe Express, Figma Community and Envato category sizes: Envato's listing page shows no counts; Figma Community and Adobe Express were not reached. No data.
- Search-volume data (for example keyword volumes for "Instagram post template", "pitch deck template", "flyer template"): not obtained.
- Usage data from AI design tools themselves on which formats are requested: none found.
