import Head from 'next/head';

import Footer from 'components/footer';
import Navbar from 'components/navbar';

export default function Home() {
  return (
    <>
      <Head>
        <title>Legal &ndash; Attempt This Online</title>
      </Head>
      <div className="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white relative flex flex-col">
        <Navbar />
        <main className="my-3 px-8 md:container md:mx-auto flex-grow pb-2">
          <article id="privacy-policy">
            <h1 className="mb-4 text-4xl font-bold">Privacy Policy</h1>
            <aside className="text-justify italic mb-3">
              <a href="https://github.com/pxeger/attempt_this_online/commits/main/frontend/pages/legal.tsx" className="underline text-blue-500">
                Last Updated: April 9th, 2021
              </a>
            </aside>
            <p className="text-justify mb-3">
              When you visit the website, the following data are collected:
            </p>
            <ul className="list-disc ml-6 my-2">
              <li>
                Your IP address. This is used to implement rate-limiting to prevent exhaustion of
                resources. A unique, hashed, and salted version of your IP address is stored for up
                to 1 year, and is never associated with any data except the amount of resources you
                use. Your IP address in an unhashed form may be stored for up to 1 month for
                security reasons.
              </li>
              <li>
                Any data you submit in the Run form (code, input, etc.). These are only used to
                process your code execution request, and immediately deleted after execution has
                completed.
              </li>
            </ul>
            <p className="text-justify mb-3">
              Any information you save on the Preferences page is stored locally in your browser
              and never shared with anyone.
            </p>
            <p className="text-justify mb-3">
              This Privacy Policy may change from time to time. If/when it does, a notification dot
              will be displayed next to the &quot;Legal&quot; link in the footer for a few weeks
              before and after. (if I remember, that is - I can&apos;t guarantee anything). You can
              also check the history of this policy page at any time
              {' '}
              <a href="https://github.com/pxeger/attempt_this_online/commits/main/frontend/pages/legal.tsx" className="underline text-blue-500">
                on GitHub
              </a>
              , and there&apos;s probably a way to get email updates there as well I reckon.
            </p>
          </article>
          <hr className="border-gray-400 dark:border-gray-700 my-4" />
          <article id="terms-of-use">
            <h1 className="mb-4 text-4xl font-bold">Terms of Use</h1>
            <aside className="text-justify italic mb-3">
              Parts of this Terms of Use document have been adapted from
              {' '}
              <a href="https://docs.github.com/en/github/site-policy" className="underline text-blue-500">
                GitHub&apos;s policies
              </a>
              , which are
              {' '}
              <a href="https://github.com/github/site-policy#license" className="underline text-blue-500">
                licensed under
              </a>
              {' '}
              the
              {' '}
              <a href="https://github.com/github/site-policy/blob/main/LICENSE.md" className="underline text-blue-500">
                Creative Commons Zero 1.0 Universal
              </a>
              {' '}
              licence .
            </aside>
            <h2 className="mb-3 text-2xl font-bold">Table of Contents</h2>
            <ol className="list-decimal ml-6 mb-3">
              <li>Definitions</li>
              <li>Software Licence Agreement</li>
              <li>Acceptable Use</li>
              <li>User-Generated Content</li>
              <li>Disclaimer of Warranties</li>
              <li>Limitation of Liability</li>
              <li>Changes to These Terms</li>
              <li>Footnotes</li>
            </ol>
            <h2 className="mb-3 text-2xl font-bold" id="definitions">1. Definitions</h2>
            <dl>
              <dt className="font-bold">&quot;the Agreement&quot;</dt>
              <dd className="ml-8 mb-2">
                Refers, collectively, to all the terms, conditions, notices contained or referenced
                in this document (the &quot;Terms of Use&quot;, &quot;Terms of Service&quot;, or
                &quot;Terms&quot;) and/or located on the web page
                {' '}
                <a href="https://ato.pxeger.com/legal" className="underline text-blue-500">
                  <code>https://ato.pxeger.com/legal</code>
                </a>
                .
              </dd>
              <dt className="font-bold">&quot;the Software&quot;</dt>
              <dd className="ml-8 mb-2">
                The code and other resources provided in the Attempt This Online repository located
                at
                {' '}
                <a href="https://github.com/pxeger/attempt_this_online" className="underline">
                  <code>https://github.com/pxeger/attempt_this_online</code>
                </a>
                {' '}
                and/or elsewhere. Use of the Software is governed by the
                {' '}
                <a
                  href="https://github.com/pxeger/attempt_this_online/blob/main/LICENCE.txt"
                  className="underline text-blue-500"
                >
                  Software Licence Agreement
                </a>
                , which is the
                {' '}
                <a href="https://www.gnu.org/licenses/agpl-3.0.en.html" className="underline text-blue-500">
                  GNU Affero General Public License 3.0
                </a>
                .
              </dd>
              <dt className="font-bold">&quot;the User&quot;</dt>
              <dt className="font-bold">&quot;You&quot;</dt>
              <dd className="ml-8 mb-2">
                The individual person, company, or organization that has visited or is using the
                Website or Service; that accesses or uses any part of the Service; or that
                directs the use of the Service in the performance of its functions. A User must
                be at least 13 years of age. Special terms may apply for business or government
                Users.
              </dd>
              <dt className="font-bold">&quot;Content&quot;</dt>
              <dt className="font-bold">&quot;Your Content&quot;</dt>
              <dt className="font-bold">&quot;User-Generated Content&quot;</dt>
              <dd className="ml-8 mb-2">
                Content featured or displayed through the Website, including without limitation
                code, text, data, articles, images, photographs, graphics, software, applications,
                packages, designs, features, and other materials that are available on the Website
                or otherwise available through the Service. &quot;Content&quot; also includes
                Services. &quot;User-Generated Content&quot; is Content, written or otherwise,
                created or uploaded by Users. &quot;Your Content&quot; is Content that you create
                or own.
              </dd>
              <dt className="font-bold">&quot;the Website&quot;</dt>
              <dt className="font-bold">&quot;the Service&quot;</dt>
              <dd className="ml-8 mb-2">
                The Attempt This Online website at
                {' '}
                <code>ato.pxeger.com</code>
                {' '}
                and all services
                provided therein. This does not include the Software itself, but only the
                instance of the Software made available at
                {' '}
                <code>ato.pxeger.com</code>
              </dd>
              <dt className="font-bold">&quot;Me&quot;</dt>
              <dt className="font-bold">&quot;I&quot;</dt>
              <dd className="ml-8 mb-2">
                Patrick Reader, the primary operator of the Service, and my affiliates,
                contractors, licensors, officers, agents, and/or employees.
              </dd>
            </dl>
            <h2 className="mb-3 text-2xl font-bold" id="licence">2. Software Licence Agreement</h2>
            <p className="mb-2 text-justify">
              In order to use the Service, you must agree to the terms of the
              {' '}
              <a href="https://github.com/pxeger/attempt_this_online/blob/main/LICENCE.txt" className="underline text-blue-500">
                Software Licence Agreement
              </a>
              , which is the
              {' '}
              <a href="https://www.gnu.org/licenses/agpl-3.0.en.html" className="underline text-blue-500">
                GNU Affero General Public License 3.0
              </a>
              .
            </p>
            <h2 className="mb-3 text-2xl font-bold" id="acceptable-use">3. Acceptable Use</h2>
            <h3 className="mb-3 text-xl font-bold" id="access-requirements">Access Requirements</h3>
            <p className="text-justify mb-2">
              You must be at least 13 years old to use the Service. The Service is not targeted to
              children under 13, and I do not permit any Users under 13 to use the Service. If I
              learn of any User under the age of 13, I will terminate that User&apos;s access to
              the Service immediately.
            </p>
            <p className="text-justify mb-2">
              Unless I grant you explicit written or electronic permission, you must not use the
              Service, except directly through the web page at
              {' '}
              <a href="https://ato.pxeger.com/run" className="underline text-blue-500">
                <code>https://ato.pxeger.com/run</code>
              </a>
              {' '}
              as a result of your manual user interaction with the web page. The Software
              includes an &quot;API&quot;, which is a programmatic interface for interacting with
              the Service, but this is for use only by the Service internally and by Users with
              explicit permission from me. You are not permitted to use the API otherwise, and your
              access to the Service may be restricted if I believe you are doing so.
              <a href="#footnote-1" className="px-1 underline text-blue-500"><sup>1</sup></a>
            </p>
            <p className="text-justify mb-2">
              I retain the right to limit or remove your access to the Service, at any time, for
              any reason, with or without notice.
            </p>
            <h3 className="mb-3 text-xl font-bold" id="compliance">Compliance</h3>
            <p className="text-justify mb-2">
              You are responsible for the compliance of your use of the Service with all applicable
              laws and contracts.
            </p>
            <h3 className="mb-3 text-xl font-bold" id="unacceptable-content">Unacceptable Content</h3>
            <p className="text-justify mb-2">
              You may not use the Service to host or transmit content that contains, promotes or
              attempts to promote, or is in itself:
            </p>
            <ul className="list-disc ml-6 my-2">
              <li className="text-justify">
                harassment, abuse, hate speech, or discrimination towards any person or group of
                people; or
              </li>
              <li className="text-justify">
                personal information or any other content that violates the privacy of any third
                party without their permission; or
              </li>
              <li className="text-justify">
                content that impersonates any person or other legal entity; or
              </li>
              <li className="text-justify">
                defamation of any party; or
              </li>
              <li className="text-justify">
                content that seeks to promote a political or religious interest; or
              </li>
              <li className="text-justify">
                content that is intentionally false or deceptive which is likely to adversely affect
                the public interest, including public health and safety and electoral integrity; or
              </li>
              <li className="text-justify">
                fraudulent activity such as scams or phishing; or
              </li>
              <li className="text-justify">
                content that incites, glorifies, or positively depicts violence; or
              </li>
              <li className="text-justify">
                content that is pornographic or sexually obscene; or
              </li>
              <li className="text-justify">
                content that infringes the intellectual property rights of any party (including
                copyright, trademarks, patents, trade secrets, etc.); or
              </li>
              <li className="text-justify">
                malware or software exploits; or
              </li>
              <li className="text-justify">
                advertising, spam, or excessive or bulk commercial content; or
              </li>
              <li className="text-justify">
                multi-level marketing businesses; or
              </li>
              <li className="text-justify">
                production, processing, promotion, sale, procurement, or consumption of raw
                tomatoes; or
              </li>
              <li className="text-justify">
                manufacturing, promotion, sale, procurement, or use of weapons or explosives; or
              </li>
              <li className="text-justify">
                any other content or activity that is illegal in the United Kingdom of Great Britan
                and Northern Ireland (the country where I am based), or in the Republic of Finland
                (the country where the Service is hosted), or in the Federal Republic of Germany
                (the country where Hetzner Online GmbH, the company which hosts the servers that
                host the Service, is based), under any law, statute, or treaty, at any level of
                government.
              </li>
            </ul>
            <h3 className="mb-3 text-xl font-bold" id="unacceptable-conduct">Unacceptable Conduct</h3>
            <p className="text-justify mb-3">You may not use or attempt to use the service for:</p>
            <ul className="list-disc ml-6 my-2">
              <li className="text-justify">
                the mining of cryptocurrency; or
              </li>
              <li className="text-justify">
                any activity that places deliberate excessive strain on computational resources
                (such as CPU share, memory usage, network bandwidth, disk space, etc.)
              </li>
              <li className="text-justify">
                use of the Service&apos;s servers to disrupt or gain unauthorised access to any
                service, network, or data
                <a href="#footnote-2" className="px-1 underline text-blue-500"><sup>2</sup></a>
                ; or
              </li>
              <li className="text-justify">
                any other activity unrelated to the demonstration of computer programs for
                educational or recreational purposes; or
              </li>
              <li className="text-justify">
                any other activity that is illegal in the United Kingdom of Great Britan and
                Northern Ireland (the country where I am based), or in the Republic of Finland (the
                country where the Service is hosted), or in the Federal Republic of Germany (the
                country where Hetzner Online GmbH, the company which hosts the servers that host
                the Service, is based), under any law, statute, or treaty, at any level of
                government.
              </li>
            </ul>
            <h2 className="mb-3 text-2xl font-bold" id="content">4. User-Generated Content</h2>
            <h3 className="mb-3 text-xl font-bold" id="content-responsibility">
              Responsibility for User-Generated Content
            </h3>
            <p className="text-justify mb-3">
              You may create or upload User-Generated Content while using the Service. You are
              solely responsible for the content of, and for any harm resulting from, any
              User-Generated Content that you post, upload, link to or otherwise make available via
              the Service, regardless of the form of that Content. I am not responsible for any
              public display or misuse of your User-Generated Content.
            </p>
            <h3 className="mb-3 text-xl font-bold" id="content-removal">
              I May Remove Content
            </h3>
            <p className="text-justify mb-3">
              I have the right to refuse or remove, with or without notice, any User-Generated
              Content that, in my sole discretion, violates any laws or any of my terms or policies,
              or for any other reason.
            </p>
            <h3 className="mb-3 text-xl font-bold" id="content-rights">
              Ownership of Content, Right to Post, and Licence Grants
            </h3>
            <p className="text-justify mb-3">
              You retain ownership of and responsibility for Your Content. If you&apos;re posting
              anything you did not create yourself or do not own the rights to, you agree that you
              are responsible for any Content you post; that you will only submit Content that you
              have the right to post; and that you will fully comply with any third party licences
              relating to Content you post.
            </p>
            <p className="text-justify mb-3">
              You grant me and my legal successor(s) the right to store, archive, parse, and
              display Your Content, and make incidental copies, as necessary to provide the
              Service, including improving the Service over time. This licence includes the right
              to do things like copy it to my database and make backups; show it to you and other
              users; parse it into a search index or otherwise analyze it on my servers; share it
              with other users; and perform it, in case Your Content is something like music or
              video. You understand that you will not receive any payment for any of these rights.
              This licence will end when you remove Your Content from my servers.
            </p>
            <h2 className="mb-3 text-2xl font-bold" id="disclaimer">5. Disclaimer of Warranties</h2>
            <p className="text-justify mb-3">
              I provide Service &quot;as is&quot; and &quot;as available,&quot; without warranty of
              any kind. Without limiting this, I expressly disclaim all warranties, whether
              express, implied or statutory, regarding the Service including without limitation
              any warranty of merchantability, fitness for a particular purpose, title, security,
              accuracy and non-infringement.
            </p>
            <p className="text-justify mb-3">
              I do not warrant that the Service will meet your requirements; that the Service will
              be uninterrupted, timely, secure, or error-free; that the information provided
              through the Service is accurate, reliable or correct; that any defects or errors will
              be corrected; that the Service will be available at any particular time or location;
              or that the Service is free of viruses or other harmful components. You assume full
              responsibility and risk of loss resulting from your downloading and/or use of files,
              information, content or other material obtained from the Service.
            </p>
            <h2 className="mb-3 text-2xl font-bold" id="liability">6. Limitation of Liability</h2>
            <p className="text-justify mb-3">
              You understand and agree that I will not be liable to you or any third party for any
              loss of profits, use, goodwill, or data, or for any incidental, indirect, special,
              consequential or exemplary damages, however arising, that result from:
            </p>
            <ul className="list-disc ml-6 my-2">
              <li className="text-justify">
                the use, disclosure, or display of your User-Generated Content; or
              </li>
              <li className="text-justify">
                your use or inability to use the Service; or
              </li>
              <li className="text-justify">
                any modification, price change, suspension or discontinuance of the Service; or
              </li>
              <li className="text-justify">
                the Service generally or the software or systems that make the Service available;
                or
              </li>
              <li className="text-justify">
                unauthorized access to or alterations of your transmissions or data; or
              </li>
              <li className="text-justify">
                statements or conduct of any third party on the Service; or
              </li>
              <li className="text-justify">
                any other user interactions that you input or receive through your use of the
                Service; or
              </li>
              <li className="text-justify">
                any other matter relating to the Service.
              </li>
            </ul>
            <p className="text-justify mb-3">
              My liability is limited whether or not I have been informed of the possibility of
              such damages, and even if a remedy set forth in this Agreement is found to have
              failed of its essential purpose. I will have no liability for any failure or delay
              due to any reason, including but not limited to matters beyond my reasonable control.
            </p>
            <h2 className="mb-3 text-2xl font-bold" id="changes">7. Changes to These Terms</h2>
            <p className="text-justify mb-3">
              I reserve the right, at my sole discretion, to amend these Terms of Service at any
              time with or without notice, and will update these Terms of Service in the event of
              any such amendments. I reserve the right at any time and from time to time to modify
              or discontinue, temporarily or permanently, the Service (or any part of it) with or
              without notice.
            </p>
            <h2 className="mb-3 text-2xl font-bold" id="misc">8. Miscellaneous</h2>
            <p className="text-justify mb-3">
              Except to the extent applicable law provides otherwise, this Agreement between you
              and me and any access to or use of the Service are governed by the laws of England
              and of the United Kingdom, without regard to conflict of law provisions. You and I
              agree to submit to the exclusive jurisdiction and venue of the courts of England in
              the United Kingdom.
            </p>
            <p className="text-justify mb-3">
              I may assign or delegate these Terms of Use, in whole or in part, to any person or
              entity at any time with or without your consent, including the licence grant in
              Section 4. You may not assign or delegate any rights or obligations under the Terms
              of Use without my prior written consent, and any unauthorized assignment and
              delegation by you is void.
            </p>
            <p className="text-justify mb-3">
              Throughout this Agreement, each section includes titles as brief summaries of the
              following terms and conditions. Section 9, &quot;Footnotes&quot;, also contains
              some information for readers. These footnotes and section titles are not legally
              binding.
            </p>
            <p className="text-justify mb-3">
              If any part of this Agreement is held invalid or unenforceable, that portion of the
              Agreement will be construed to reflect the parties’ original intent. The remaining
              portions will remain in full force and effect. Any failure on my part to enforce any
              provision of this Agreement will not be considered a waiver of my right to enforce
              such provision. My rights under this Agreement will survive any termination of this
              Agreement.
            </p>
            <p className="text-justify mb-3">
              This Agreement may only be modified by an electronic or written amendment signed by
              me or an authorized representative of me, or by my posting of a revised version in
              accordance with Section 7, &quot;Changes to These Terms&quot;. These Terms of Use
              represent the complete and exclusive statement of the agreement between you and me.
              This Agreement supersedes any proposal or prior agreement oral or written, and any
              other communications between you and me relating to the subject matter of these terms
              including any confidentiality or nondisclosure agreements.
            </p>
            <h2 className="mb-3 text-2xl font-bold" id="footnotes">9. Footnotes</h2>
            <aside className="text-justify italic">
              The following footnotes are for informational purposes only and are not part of the
              Terms of Use.
            </aside>
            <ol className="list-decimal ml-6 my-2">
              <li className="text-justify" id="footnote-1">
                To request API access,
                {' '}
                <a href="https://www.pxeger.com/#contact-me" className="text-blue-500 underline">
                  contact me
                </a>
                .
              </li>
              <li className="text-justify" id="footnote-2">
                If you want to find security vulnerabilities in Attempt This Online, please do so
                only on your own privately hosted instance or other instances where you have
                permission to do so. See the project&apos;s
                {' '}
                <a href="https://github.com/pxeger/attempt_this_online/security/policy" className="underline text-blue-500">
                  Security Policy
                </a>
                {' '}
                for more information.
              </li>
            </ol>
          </article>
        </main>
        <Footer noLegalLink />
      </div>
    </>
  );
}
